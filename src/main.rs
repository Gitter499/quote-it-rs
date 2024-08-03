use anyhow::{Context, Ok};
use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::{Local, NaiveDateTime, Timelike};
use clap::{ArgAction, Parser, Subcommand};
use homedir::my_home;
use polodb_core::bson::*;
use polodb_core::{Collection, Database};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{fmt::Display, fs, path::PathBuf};

fn parse_date(arg: &str) -> anyhow::Result<DateTime> {
    let naive_date_time =
        NaiveDateTime::parse_from_str(&format!("{}T0:00:00", arg), "%m-%d-%YT%H:%M:%S")?;
    Ok(DateTime::from_chrono(naive_date_time.and_utc()))
}

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    author,
    long_about = "A quoting utility in the terminal"
)]
pub struct CLI {
    pub quote: Option<String>,
    /// Specify an author
    #[arg(short, long)]
    pub author: Option<String>,
    /// Add a date
    #[arg(short, long,action=ArgAction::SetTrue)]
    pub date: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Lists quotes stored on the device
    List {
        /// Lists quotes made by specified author
        #[arg(long, short)]
        author: Option<String>,
        /// List quotes by date (make sure to provide date in the following format: mm-dd-yyyy)
        #[arg(long, short, value_parser = parse_date)]
        date: Option<DateTime>,
    },
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Quote {
    quote: String,
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    date: Option<DateTime>,
}

pub trait ToChronoDateFormatted {
    fn to_date_formatted(&self) -> DelayedFormat<StrftimeItems>;
}

impl ToChronoDateFormatted for DateTime {
    fn to_date_formatted(&self) -> DelayedFormat<StrftimeItems> {
        self.to_chrono().format("%m-%d-%Y")
    }
}

impl Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut quote_string = String::from_str(&format!("{:#?}", self.quote)).unwrap();
        if let Some(quote_author) = self.author.as_ref() {
            quote_string.push_str(&format!("\n  - {}", quote_author));
        };

        if let Some(quote_timestamp) = self.date.as_ref() {
            quote_string.push_str(&format!(" on {}", quote_timestamp.to_date_formatted()))
        };
        quote_string.push_str("\n------------");

        writeln!(f, "{}", quote_string)
    }
}

fn main() -> anyhow::Result<()> {
    // println!("Hello Quote it!");
    // Get CLI args

    let args = CLI::parse();

    let db = Database::open_file(get_file_path()?).context("Database file search failed")?;

    let quotes: Collection<Quote> = db.collection("quotes");

    if let Some(c) = args.command {
        match c {
            Commands::List { author, date } => {
                let mut doc = bson::Document::new();

                if author.is_some() {
                    doc.insert("author", &author);
                }

                if date.is_some() {
                    doc.insert(
                        "date",
                        doc! {
                            "$lte": &date
                        },
                    );
                }

                let found_quotes = quotes
                    .find(if !doc.is_empty() { Some(doc) } else { None })?
                    .collect::<polodb_core::Result<Vec<Quote>>>()?;

                if found_quotes.len() == 0 {
                    let mut message = String::from("No quotes found");

                    if let Some(author) = author.as_ref() {
                        message.push_str(&format!(" by {}", author));
                    }

                    if let Some(date) = date.as_ref() {
                        message.push_str(&format!(" on {}", date.to_date_formatted()));
                    }

                    if date.is_none() && author.is_none() {
                        message.push_str(". Try creating a quote with `quote-it <QUOTE>`");
                    }

                    println!("{}", message);
                }

                for quote in found_quotes {
                    println!("{}", quote)
                }
            }
        }
    };

    if let Some(quote_content) = args.quote {
        let mut new_quote = Quote::default();

        new_quote.quote = quote_content;
        new_quote.author = args.author;

        if args.date {
            // Yes this solution is jank, so is everything in this repo to do with dates/time
            let local = Local::now().naive_local();
            let date_zeroed_time = local
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                .and_utc();

            let bson_date = DateTime::from_chrono(date_zeroed_time);
            new_quote.date = Some(bson_date);
        }
        quotes.insert_one(new_quote)?;
    }

    Ok(())
}

fn get_file_path() -> anyhow::Result<PathBuf> {
    let mut file_path = my_home()?.unwrap();

    file_path.push(".quote-it");

    if !file_path.exists() {
        fs::create_dir(&file_path).context("Failed to create quotes directory")?;
    }

    file_path.push("quotes.db");

    if !file_path.exists() {
        fs::File::create(&file_path).context("Quote file creation failed")?;
    }

    Ok(file_path)
}
