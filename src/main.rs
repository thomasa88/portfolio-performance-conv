use anyhow::Result;
use clap::Parser;

mod avanza;

#[derive(Parser, Debug)]
struct Args {
    file: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let output = args.file.with_extension("pp.csv");
    avanza::convert(&args.file, &output)?;
    Ok(())
}
