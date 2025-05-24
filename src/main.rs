use std::{
    path::{Path, PathBuf},
    pin::pin,
    sync::Arc,
};

use clap::Parser;
use colored::Colorize;
use iced::{
    Element, Subscription,
    futures::{SinkExt, Stream, StreamExt},
    stream::try_channel,
    widget,
};

mod avanza;
mod pp;
mod types;
mod yahoo_symbol;

#[derive(Default)]
struct Settings {
    path: String,
    log: iced::widget::text_editor::Content,
    running: bool,
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    SelectFile,
    Convert,
    ConversionDone(Result<(), ConversionError>),
    Log(Result<ConversionProgress, ConversionError>),
    EditLog(widget::text_editor::Action),
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
                self.log = widget::text_editor::Content::new();
                self.running = true;
            }
            Message::Log(progress) => {
                let Ok(progress) = progress else {
                    println!("Unhandled progress error");
                    return;
                };
                match progress {
                    ConversionProgress::Log(msg) => {
                        self.log.perform(widget::text_editor::Action::Edit(
                            widget::text_editor::Edit::Paste(Arc::new(msg)),
                        ));
                        self.log.perform(widget::text_editor::Action::Edit(
                            widget::text_editor::Edit::Enter,
                        ));
                    }
                    ConversionProgress::Done => {
                        self.running = false;
                    }
                }
            }
            Message::ConversionDone(_) => {}
            Message::EditLog(action) => {
                // Make the text box read-only
                if !action.is_edit() {
                    self.log.perform(action);
                }
            }
        }
    }

    // iced doc: "Try to treat these functions as declarative, stateless functions."
    fn subscription(&self) -> Subscription<Message> {
        // Conversion::subscription
        if self.running {
            // This gets called on every event...
            // so make sure to do as little as possible. It looks like the channel
            // created in convert() only gets called once.
            let convert_id = 1;
            let path = Path::new(&self.path).to_owned();
            Subscription::run_with_id(convert_id, convert(path).map(Message::Log))
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        use iced::widget::*;
        widget::column![
            row![
                text("Transaktionsfil:"),
                text_input("", &self.path)
                    .on_input(Message::PathChanged)
                    .on_submit(Message::Convert),
                button("Välj CSV...").on_press(Message::SelectFile),
            ]
            .spacing(5),
            row![
                horizontal_space(),
                button("Konvertera").on_press(Message::Convert),
                horizontal_space()
            ],
            text_editor(&self.log)
                .height(iced::Length::Fill)
                .size(13)
                .on_action(Message::EditLog)
        ]
        .spacing(5)
        .padding(5)
        .into()
    }
}

#[derive(Debug, Clone)]
enum ConversionError {
    TBD,
}

#[derive(Debug, Clone)]
enum ConversionProgress {
    // Msg(String),
    Log(String),
    Done,
}

#[derive(Parser, Debug)]
struct Args {
    file: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    iced::application(
        "Portfolio Performance Converter",
        Settings::update,
        Settings::view,
    )
    .window_size(iced::Size::new(850., 400.))
    .subscription(Settings::subscription)
    .run()?;
    return Ok(());

    // let args = Args::parse();
    // convert(&args.file)
}

fn convert(input_path: PathBuf) -> impl Stream<Item = Result<ConversionProgress, ConversionError>> {
    try_channel(3, async move |mut output| {
        output
            .send(ConversionProgress::Log(format!(
                "Konverterar {}...",
                input_path.display()
            )))
            .await
            .unwrap();
        let portfolio_output = input_path.with_extension("pp-portfolio-transactions.csv");
        let account_output = input_path.with_extension("pp-account-transactions.csv");
        let mut writer = pp::CsvWriter::new(&portfolio_output, &account_output)
            .map_err(|e| ConversionError::TBD)?;
        {
            let mut s = pin!(avanza::convert(&input_path, &mut writer));
            while let Some(Ok(log_msg)) = s.next().await {
                output.send(ConversionProgress::Log(log_msg)).await.unwrap();
            }
            // s.next();
        }

        let mut deps: Vec<_> = writer.cash_accounts().iter().collect();
        deps.sort();
        let mut secs: Vec<_> = writer.security_accounts().iter().collect();
        secs.sort();
        println!();
        output
            .send(ConversionProgress::Log(format!(
                "Add the below accounts before importing the CSV files."
            )))
            .await
            .unwrap();
        output.send(ConversionProgress::Log(format!("{}", "Failing to add all accounts will likely result in transactions silently being connected to another account.".red()))).await.unwrap();
        println!();
        output
            .send(ConversionProgress::Log(format!("Securities accounts:")))
            .await
            .unwrap();
        for account in secs {
            output
                .send(ConversionProgress::Log(format!("* {account}")))
                .await
                .unwrap();
        }
        println!();
        output
            .send(ConversionProgress::Log(format!(
                "Deposit accounts (Reference accounts):"
            )))
            .await
            .unwrap();
        for account in deps {
            output
                .send(ConversionProgress::Log(format!("* {account}")))
                .await
                .unwrap();
        }
        println!();
        output
            .send(ConversionProgress::Log(format!(
                "Portfolio transactions: {}",
                portfolio_output.display()
            )))
            .await
            .unwrap();
        output
            .send(ConversionProgress::Log(format!(
                "Account transactions: {}",
                account_output.display()
            )))
            .await
            .unwrap();

        #[cfg(target_os = "windows")]
        {
            // The user did not open the program in a terminal, so pause so that they can read the output.
            // println!("{}", "\nPress enter to exit.".green());
            // std::io::stdin().read_line(&mut String::new()).ok();
        }

        output.send(ConversionProgress::Done).await.unwrap();
        Ok(())
    })
}
