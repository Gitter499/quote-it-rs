use anyhow::{Context, Ok};
use chrono::{Local, NaiveDateTime, Timelike};
use clap::{ArgAction, Parser, Subcommand};
use homedir::my_home;
use polodb_core::bson::*;
use polodb_core::{Collection, Database};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{fmt::Display, fs, path::PathBuf};

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

fn parse_date(arg: &str) -> anyhow::Result<DateTime> {
    let naive_date_time =
        NaiveDateTime::parse_from_str(&format!("{}T0:00:00", arg), "%m-%d-%YT%H:%M:%S")?;
    Ok(DateTime::from_chrono(naive_date_time.and_utc()))
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Quote {
    quote: String,
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    date: Option<DateTime>,
}

impl Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut quote_string = String::from_str(&format!("{:#?}", self.quote)).unwrap();
        if let Some(quote_author) = self.author.as_ref() {
            quote_string.push_str(&format!("\n  - {}", quote_author));
        };

        if let Some(quote_timestamp) = self.date.as_ref() {
            // Due to serde limitations with chrono timezones, we must convert the time during print, although it is stored
            // as utc in the db. We must do this conversion for import/export as well
            let converted_date: chrono::DateTime<Local> =
                chrono::DateTime::from(quote_timestamp.to_chrono());
            quote_string.push_str(&format!(" on {}", converted_date.format("%m-%d-%Y")))
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
                let found_quotes = if author.is_some() && date.is_some() {
                    quotes.find(doc! {
                        "author": author.as_ref().unwrap(),
                        "date": { "$lte": &date }
                    })
                } else if author.is_some() {
                    quotes.find(doc! {
                        "author": author.as_ref().unwrap()
                    })
                } else if date.is_some() {
                    quotes.find(doc! {
                        "date": {
                            "$lte": &date
                        }
                    })
                } else {
                    quotes.find(None)
                };
                // Very jank way of doing this but iterators leave me no choice
                let mut i = 0;
                for quote in found_quotes? {
                    i += 1;
                    println!("{}", quote?)
                }

                if i == 0 {
                    let mut message = String::from("No quotes found");
                    let formatted_date = date.as_ref().map(|d| d.to_chrono().format("%m-%d-%Y"));
                    if author.is_some() && date.is_some() {
                        message.push_str(&format!(
                            " by {} on {}",
                            author.as_ref().unwrap(),
                            formatted_date.as_ref().unwrap()
                        ));
                    } else if author.is_some() {
                        message.push_str(&format!(" by {}", author.as_ref().unwrap()));
                    } else if date.is_some() {
                        message.push_str(&format!(" on {}", formatted_date.as_ref().unwrap()));
                    } else {
                        message.push_str(". Try creating a quote with `quote-it <QUOTE>`")
                    }
                    println!("{}", message);
                }
            }
        }
    };

    if let Some(quote_content) = args.quote {
        let mut new_quote = Quote::default();

        new_quote.quote = quote_content;
        if let Some(author) = args.author {
            new_quote.author = Some(author);
        }

        if args.date {
            // Yes this solution is jank, so is everything in this repo to do with dates/time
            // I am thinking of converting to use plain milliseconds since unix epoch but right now this works
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
