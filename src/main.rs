use anyhow::Result;
use clap::Parser;
use colored::Colorize;

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
    let portfolio_output = args.file.with_extension("pp-portfolio-transactions.csv");
    let account_output = args.file.with_extension("pp-account-transactions.csv");
    let mut writer = pp::CsvWriter::new(&portfolio_output, &account_output)?;
    avanza::convert(&args.file, &mut writer)?;

    let mut deps: Vec<_> = writer.cash_accounts().iter().collect();
    deps.sort();
    let mut secs: Vec<_> = writer.security_accounts().iter().collect();
    secs.sort();
    println!();
    println!("Add the below accounts before importing the CSV files.");
    println!("{}", "Failing to add all accounts will likely result in transactions silently being connected to another account.".red());
    println!();
    println!("Securities accounts:");
    for account in secs {
        println!("* {account}");
    }
    println!();
    println!("Deposit accounts (Reference accounts):");
    for account in deps {
        println!("* {account}");
    }
    println!();
    println!("Portfolio transactions: {}", portfolio_output.display());
    println!("Account transactions: {}", account_output.display());

    #[cfg(target_os = "windows")]
    {
        // The user did not open the program in a terminal, so pause so that they can read the output.
        println!("{}", "\nPress enter to exit.".green());
        std::io::stdin().read_line(&mut String::new()).ok();
    }

    Ok(())
}
