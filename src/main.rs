use anyhow::Result;
use clap::Parser;

mod avanza;
mod pp;
mod types;
mod yahoo_symbol;

#[derive(Parser, Debug)]
struct Args {
    file: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let portfolio_output = args.file.with_extension("pp-portfolio.csv");
    let account_output = args.file.with_extension("pp-account.csv");
    let mut writer = pp::CsvWriter::new(portfolio_output, account_output)?;
    avanza::convert(&args.file, &mut writer)?;
    Ok(())
}
