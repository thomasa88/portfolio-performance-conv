use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use iced::{
    Element, Subscription,
    futures::{SinkExt, Stream, StreamExt, TryStreamExt, channel::mpsc::Sender},
    stream::{channel, try_channel},
    widget,
};
use tokio::pin;

mod avanza;
mod pp;
mod types;
mod yahoo_symbol;

struct Settings {
    path: String,
    log: iced::widget::text_editor::Content,
    running: bool,
    selecting_file: bool,
    status: String,
    conv_count: Option<usize>,
    conv_total: Option<usize>,
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    SelectFile,
    FileSelected(Option<PathBuf>),
    Convert,
    ConversionDone(Result<(), ConversionError>),
    Log(Result<ConversionProgress, ConversionError>),
    EditLog(widget::text_editor::Action),
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: Default::default(),
            log: Default::default(),
            running: false,
            selecting_file: Default::default(),
            status: "Välj en fil att konvertera".to_string(),
            conv_count: None,
            conv_total: None,
        }
    }
}

impl Settings {
    fn update(&mut self, message: Message) {
        match message {
            Message::PathChanged(path) => {
                self.path = path;
            }
            Message::SelectFile => {
                self.selecting_file = true;
            }
            Message::Convert => {
                self.status = format!("Konverterar...");
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
                    ConversionProgress::Count(count) => {
                        self.conv_count = Some(count);
                    }
                    ConversionProgress::Total(total) => {
                        self.conv_total = Some(total);
                    }
                    ConversionProgress::Done => {
                        self.running = false;
                        self.status = format!("Klar!");
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
            Message::FileSelected(path) => {
                self.selecting_file = false;
                if let Some(path) = path {
                    self.path = path.to_string_lossy().into_owned();
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
        } else if self.selecting_file {
            let select_file_id = 2;
            Subscription::run_with_id(select_file_id, select_file())
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        use iced::widget::*;
        let mut convert_btn = button("Konvertera");
        if !self.running {
            convert_btn = convert_btn.on_press(Message::Convert);
        }
        let count_text = if let Some(count) = self.conv_count {
            if let Some(total) = self.conv_total {
                format!("{count}/{total}")
            } else {
                format!("{count}")
            }
        } else {
            String::new()
        };
        widget::column![
            row![
                text("Transaktionsfil:").align_y(iced::alignment::Vertical::Center),
                text_input("", &self.path)
                    .on_input(Message::PathChanged)
                    .on_submit(Message::Convert),
                button("Välj CSV...").on_press(Message::SelectFile),
                convert_btn,
            ]
            .spacing(5),
            row![text(&self.status), horizontal_space(), text(count_text),],
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
    Log(String),
    Count(usize),
    Total(usize),
    Done,
}

#[derive(Parser, Debug)]
struct Args {
    /// Fil att konvertera
    file: Option<std::path::PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(input_path) = args.file {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move {
            let s = convert(input_path);
            pin!(s);
            while let Some(result) = s.next().await {
                match result {
                    Ok(progress) => {
                        match progress {
                            ConversionProgress::Log(msg) => println!("{msg}"),
                            ConversionProgress::Count(_) => (),
                            ConversionProgress::Total(_) => (),
                            ConversionProgress::Done => (),
                        };
                    }
                    Err(_) => todo!(),
                }
            }
        });
    } else {
        iced::application(
            "Portfolio Performance Converter",
            Settings::update,
            Settings::view,
        )
        .window_size(iced::Size::new(850., 400.))
        .subscription(Settings::subscription)
        .run()?;
    }
    Ok(())
}

#[derive(Clone)]
struct ProgressSender {
    sender: Sender<ConversionProgress>,
}

impl ProgressSender {
    async fn log(&mut self, msg: impl Into<String>) {
        // Errors are ignored -> "lossy logging"
        self.sender
            .send(ConversionProgress::Log(msg.into()))
            .await
            .ok();
    }

    async fn count(&mut self, value: usize) {
        self.sender
            .send(ConversionProgress::Count(value))
            .await
            .ok();
    }

    async fn total(&mut self, value: usize) {
        self.sender
            .send(ConversionProgress::Total(value))
            .await
            .ok();
    }
}

fn convert(input_path: PathBuf) -> impl Stream<Item = Result<ConversionProgress, ConversionError>> {
    try_channel(1, async move |mut output| {
        let mut progress = ProgressSender {
            sender: output.clone(),
        };
        progress
            .log(format!("Konverterar {}...", input_path.display()))
            .await;
        let portfolio_output = input_path.with_extension("pp-portfolio-transactions.csv");
        let account_output = input_path.with_extension("pp-account-transactions.csv");
        let mut writer = pp::CsvWriter::new(&portfolio_output, &account_output)
            .map_err(|e| ConversionError::TBD)?;
        avanza::convert(&input_path, &mut writer, progress.clone())
            .await
            .map_err(|e| ConversionError::TBD)?;

        let mut deps: Vec<_> = writer.cash_accounts().iter().collect();
        deps.sort();
        let mut secs: Vec<_> = writer.security_accounts().iter().collect();
        secs.sort();
        progress
            .log(format!(
                "


Lägg till följande konton i Portfolio Performance innan du importerar CSV-filerna.
Om du inte lägger in alla konton i förväg så kommer transaktioner hamna på fel konton.
"
            ))
            .await;
        progress.log(format!("Securities accounts:")).await;
        for account in secs {
            progress.log(format!("* {account}")).await;
        }
        progress
            .log(format!("\nDeposit accounts (Reference accounts):"))
            .await;
        for account in deps {
            progress.log(format!("* {account}")).await;
        }
        progress
            .log(format!(
                "\nPortfolio transactions: {}",
                portfolio_output.display()
            ))
            .await;
        progress
            .log(format!(
                "Account transactions: {}",
                account_output.display()
            ))
            .await;

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

fn select_file() -> impl Stream<Item = Message> {
    channel(1, async |mut output| {
        if let Some(path) = rfd::AsyncFileDialog::new()
            .add_filter("CSV", &["csv"])
            .pick_file()
            .await
        {
            output
                .send(Message::FileSelected(Some(path.into())))
                .await
                .unwrap();
        } else {
            // Stop the file selecting state
            output.send(Message::FileSelected(None)).await.unwrap();
        }
    })
}
