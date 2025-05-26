use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    ops::Deref,
    sync::atomic::{self, AtomicBool},
};
use tokio::{
    sync::{Mutex, RwLock, RwLockReadGuard},
    time::{Duration, Instant, sleep},
};

use crate::ProgressSender;

const CACHE_FILENAME: &str = "yahoo_cache.json";

/// Performs lookups towards Yahoo Finance.
///
/// Results are cached for a number of days, to avoid unnecessary calls to Yahoo.
/// [`Self::save_cache()`] **must be called** before dropping [`Yahoo`], to save the cache.
pub(crate) struct Yahoo {
    cache: RwLock<Cache>,
    cache_is_dirty: AtomicBool,
    /// For rate limiting
    last_fetch: Mutex<Option<Instant>>,
    progress: Option<Mutex<ProgressSender>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Cache {
    entries: HashMap<String, IsinLookup>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Security {
    pub(crate) symbol: String,
    pub(crate) exchange: String,
    /// The name is often messy. Only use it as a fallback.
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct SecurityEntry<'c> {
    isin: &'c str,
    rcache: RwLockReadGuard<'c, Cache>,
}

impl<'c> Deref for SecurityEntry<'c> {
    type Target = Vec<Security>;

    fn deref(&self) -> &Self::Target {
        // unwrap: Value was found to exist in the cache and cannot be removed as Self is holding a lock.
        &self.rcache.entries.get(self.isin).unwrap().securities
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IsinLookup {
    securities: Vec<Security>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct YahooResponse {
    quotes: Vec<YahooQuote>,
    // ..
}

#[derive(Debug, Deserialize)]
struct YahooQuote {
    /// Often cut off or has `"` at the end
    shortname: String,
    // /// Tend to miss umlaut characters
    // longname: Option<String>,
    exchange: String,
    symbol: String,
}

impl Yahoo {
    pub fn new() -> Self {
        let cache = RwLock::new(read_cache());
        Yahoo {
            last_fetch: Mutex::new(None),
            cache,
            cache_is_dirty: AtomicBool::new(false),
            progress: None,
        }
    }

    pub fn new_with_progress(progress: ProgressSender) -> Self {
        let mut y = Self::new();
        y.progress = Some(Mutex::new(progress));
        y
    }
}

impl Yahoo {
    /// Looks up the Yahoo ticker symbol(s) for the given ISIN. Multiple symbols can be returned if the security is
    /// available at multiple exchanges. The returned vec is empty if no symbols are found.
    pub async fn isin_to_symbols<'c>(&'c self, isin: &'c str) -> anyhow::Result<SecurityEntry<'c>> {
        {
            let rcache = self.cache.read().await;
            let lookup = rcache.entries.get(isin);
            if let Some(lookup) = lookup {
                if Utc::now() - lookup.updated_at < chrono::TimeDelta::days(5) {
                    return Ok(SecurityEntry { isin, rcache });
                }
            }
        }

        self.log(format!("Hämtar symbol for {isin} från Yahoo Finance:"))
            .await;

        let lookup = self.rate_limit_fetch(isin).await?;

        if lookup.securities.is_empty() {
            self.log("Ingen träff").await;
        } else {
            self.log(format!(
                "{}",
                lookup
                    .securities
                    .iter()
                    .map(|s| s.symbol.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .await;
        }

        let mut wcache = self.cache.write().await;
        wcache.entries.insert(isin.to_owned(), lookup);
        self.cache_is_dirty.store(true, atomic::Ordering::Relaxed);

        let rcache = wcache.downgrade();
        Ok(SecurityEntry {
            isin: isin,
            rcache: rcache,
        })
    }

    async fn log(&self, msg: impl Into<String>) {
        if let Some(ref progress) = self.progress {
            progress.lock().await.log(msg).await;
        }
    }

    async fn rate_limit_fetch(&self, isin: &str) -> anyhow::Result<IsinLookup> {
        // Download rate limit
        let now = Instant::now();
        let mut last_fetch = self.last_fetch.lock().await;
        if let Some(last) = *last_fetch {
            let passed = last - now;
            let wait = Duration::from_millis(500) - passed;
            sleep(wait).await;
        }
        *last_fetch = Some(now);
        fetch_securities(isin).await
    }

    /// Saves the internal cache.
    ///
    /// A separate function is required since async drop is not supported.
    pub async fn save_cache(&self) {
        let rcache = self.cache.read().await;
        write_cache(&rcache);
        self.cache_is_dirty.store(false, atomic::Ordering::Relaxed);
    }
}

impl Drop for Yahoo {
    fn drop(&mut self) {
        if self.cache_is_dirty.load(atomic::Ordering::Relaxed) {
            println!("Implementation Error: Yahoo finance cache was not saved! Call save_cache().");
        }
    }
}

async fn fetch_securities(isin: &str) -> anyhow::Result<IsinLookup> {
    let r = rand::rng().random_range(100000..=999999);
    let user_agent = format!("Mozilla/5.0 ({r})");
    let client = reqwest::ClientBuilder::new()
        .user_agent(user_agent)
        .build()?;
    let resp = client.get(format!("https://query2.finance.yahoo.com/v1/finance/search?q={isin}&lang=en-US&region=US&quotesCount=6&newsCount=3&listsCount=2&enableFuzzyQuery=false&quotesQueryId=tss_match_phrase_query&multiQuoteQueryId=multi_quote_single_token_query&newsQueryId=news_cie_vespa&enableCb=false&enableNavLinks=true&enableEnhancedTrivialQuery=true&enableResearchReports=true&enableCulturalAssets=true&enableLogoUrl=true&enableLists=false&recommendCount=5&enablePrivateCompany=true")).send().await?.text().await.unwrap();
    let resp: YahooResponse = serde_json::from_str(&resp)?;
    let now = chrono::Utc::now();
    let securities: Vec<_> = resp
        .quotes
        .into_iter()
        .map(|q| Security {
            symbol: q.symbol,
            exchange: q.exchange,
            name: q.shortname,
        })
        .collect();
    Ok(IsinLookup {
        securities,
        updated_at: now,
    })
}

fn read_cache() -> Cache {
    File::open(CACHE_FILENAME)
        .map(|f| serde_json::from_reader(BufReader::new(f)).expect("Bad cache format"))
        .unwrap_or_default()
}

fn write_cache(cache: &Cache) {
    let f = File::create(CACHE_FILENAME).unwrap();
    let writer = BufWriter::new(f);
    serde_json::to_writer_pretty(writer, &cache).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn isin_to_symbol() {
        let y = Yahoo::new();
        let sec = y.isin_to_symbols("NO0010827280").await.unwrap();
        assert_eq!(sec[0].symbol, "0P0001Q6FC.ST");
        assert_eq!(sec[0].exchange, "STO");
        drop(sec);

        let sec = y.isin_to_symbols("SE0010296574").await.unwrap();
        assert_eq!(sec[0].symbol, "ETHEREUM-XBT.ST");
        assert_eq!(sec[0].exchange, "STO");
        assert_eq!(sec[1].symbol, "SE0010296574.SG");
        assert_eq!(sec[1].exchange, "STU");
        drop(sec);

        // Make sure to make at least on non-cached look-up
        let mut y = y;
        y.cache.get_mut().entries.clear();
        let sec = y.isin_to_symbols("SE0000671919").await.unwrap();
        assert_eq!(sec[0].symbol, "0P00000LST.ST");
        assert_eq!(sec[0].exchange, "STO");
        drop(sec);
    }

    // TODO: Test cache file save and load
}
