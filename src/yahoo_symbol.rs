use std::{
    cell::{Cell, Ref, RefCell},
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter},
    time::Duration,
};

use anyhow::{anyhow, bail};
use chrono::{TimeZone, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

const CACHE_FILENAME: &str = "yahoo_cache.json";

pub(crate) struct Yahoo {
    last_lookup: Cell<Option<std::time::Instant>>,
    cache: RefCell<Cache>,
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
    /// Tend to miss umlaut characters
    longname: Option<String>,
    exchange: String,
    symbol: String,
}

impl Yahoo {
    pub fn new() -> Self {
        let cache = RefCell::new(read_cache());
        Yahoo {
            last_lookup: Cell::new(None),
            cache: cache,
        }
    }
}

impl Drop for Yahoo {
    fn drop(&mut self) {
        write_cache(&self.cache.borrow());
    }
}

impl Yahoo {
    /// Looks up the Yahoo ticker symbol(s) for the given ISIN. Multiple symbols can be returned if the security is
    /// available at multiple exchanges. The returned vec is empty if no symbols are found.
    pub fn isin_to_symbols(&self, isin: &str) -> anyhow::Result<Ref<Vec<Security>>> {
        {
            let cached = self.cache.borrow();
            let lookup = Ref::filter_map(cached, |c| c.entries.get(isin));
            if let Ok(lookup) = lookup {
                if Utc::now() - lookup.updated_at < chrono::TimeDelta::days(5) {
                    return Ok(Ref::map(lookup, |l| &l.securities));
                }
            }
        }

        // Rate limit
        let now = std::time::Instant::now();
        if let Some(last) = self.last_lookup.get() {
            let passed = last - now;
            let wait = Duration::from_secs(1) - passed;
            std::thread::sleep(wait);
        }
        self.last_lookup.replace(Some(now));
        let lookup = fetch_securities(isin)?;

        self.cache
            .borrow_mut()
            .entries
            .insert(isin.to_owned(), lookup);

        Ok(Ref::map(self.cache.borrow(), |c| {
            c.entries.get(isin).map(|l| &l.securities).unwrap()
        }))
    }
}

fn fetch_securities(isin: &str) -> anyhow::Result<IsinLookup> {
    let r = rand::rng().random_range(100000..=999999);
    let user_agent = format!("Mozilla/5.0 ({r})");
    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent(user_agent)
        .build()?;
    let resp = client.get(format!("https://query2.finance.yahoo.com/v1/finance/search?q={isin}&lang=en-US&region=US&quotesCount=6&newsCount=3&listsCount=2&enableFuzzyQuery=false&quotesQueryId=tss_match_phrase_query&multiQuoteQueryId=multi_quote_single_token_query&newsQueryId=news_cie_vespa&enableCb=false&enableNavLinks=true&enableEnhancedTrivialQuery=true&enableResearchReports=true&enableCulturalAssets=true&enableLogoUrl=true&enableLists=false&recommendCount=5&enablePrivateCompany=true")).send()?.text().unwrap();
    // dbg!(&resp);
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
        .unwrap_or(Cache::default())
}

fn write_cache(cache: &Cache) {
    let f = File::create(CACHE_FILENAME).unwrap();
    let writer = BufWriter::new(f);
    serde_json::to_writer_pretty(writer, &cache).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isin_to_symbol() {
        let y = Yahoo::new();
        let sec = y.isin_to_symbols("NO0010827280").unwrap();
        assert_eq!(sec[0].symbol, "0P0001Q6FC.ST");
        assert_eq!(sec[0].exchange, "STO");
        drop(sec);
        
        let sec = y.isin_to_symbols("SE0010296574").unwrap();
        assert_eq!(sec[0].symbol, "ETHEREUM-XBT.ST");
        assert_eq!(sec[0].exchange, "STO");
        assert_eq!(sec[1].symbol, "SE0010296574.SG");
        assert_eq!(sec[1].exchange, "STU");
        drop(sec);

        // Make sure to make at least on non-cached look-up
        y.cache.borrow_mut().entries.clear();
        let sec = y.isin_to_symbols("SE0000671919").unwrap();
        assert_eq!(sec[0].symbol, "0P00000LST.ST");
        assert_eq!(sec[0].exchange, "STO");
        drop(sec);
    }
}
