use std::path::Path;

use anyhow::{Ok, Result};
use clap::Parser;
use colored::Colorize;
use iced::{
    Element,
    widget::{Column, text_editor},
};

mod avanza;
mod pp;
mod types;
mod yahoo_symbol;

#[derive(Default)]
struct Settings {
    path: String,
    log: iced::widget::text_editor::Content,
}

#[derive(Debug, Clone)]
pub enum Message {
    PathChanged(String),
    SelectFile,
    Convert,
    Log,
}

impl Settings {
    fn update(&mut self, message: Message) {
        match message {
            Message::PathChanged(path) => {
                self.path = path;
            }
            Message::SelectFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .pick_file()
                {
                    self.path = path.to_string_lossy().into_owned();
                }
            }
            Message::Convert => {
                let path = Path::new(&self.path);
                convert(&path).unwrap()
            }
            Message::Log => todo!(),
        }
    }

    fn view(&self) -> Element<Message> {
        use iced::widget::{
            button, center, column, container, horizontal_space, row, text, text_input,
        };

        column![
            row![
                text("Transaktionsfil:"),
                text_input("", &self.path)
                    .on_input(Message::PathChanged)
                    .on_submit(Message::Convert),
                button("Bläddra...").on_press(Message::SelectFile),
            ]
            .spacing(5),
            row![
                horizontal_space(),
                button("Konvertera").on_press(Message::Convert),
                horizontal_space()
            ],
            text_editor(&self.log).height(iced::Length::Fill)
        ]
        .spacing(5)
        .into()
    }
}

#[derive(Parser, Debug)]
struct Args {
    file: std::path::PathBuf,
}

fn main() -> Result<()> {
    iced::application(
        "Portfolio Performance Converter",
        Settings::update,
        Settings::view,
    )
    .window_size(iced::Size::new(850., 400.))
    .run()?;
    return Ok(());

    let args = Args::parse();
    convert(&args.file)
}

fn convert(input_path: &Path) -> Result<()> {
    let portfolio_output = input_path.with_extension("pp-portfolio-transactions.csv");
    let account_output = input_path.with_extension("pp-account-transactions.csv");
    let mut writer = pp::CsvWriter::new(&portfolio_output, &account_output)?;
    avanza::convert(&input_path, &mut writer)?;

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
