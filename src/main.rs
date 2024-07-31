use anyhow::{Context, Ok};
use chrono::Local;
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
    /// Add a timestamp
    #[arg(short, long,action=ArgAction::SetTrue)]
    pub timestamp: bool,
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
    },
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Quote {
    quote: String,
    author: Option<String>,
    date: Option<chrono::DateTime<Local>>,
}

impl Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut quote_string = String::from_str(&format!("{:#?}", self.quote)).unwrap();
        if let Some(quote_author) = self.author.as_ref() {
            quote_string.push_str(&format!("\n  - {}", quote_author));
        };

        if let Some(quote_timestamp) = self.date.as_ref() {
            quote_string.push_str(&format!(" on {}", quote_timestamp.format("%m-%d-%Y")))
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

    match args.command {
        Some(c) => match c {
            Commands::List { author } => {
                for quote in quotes.find(if author.is_some() {
                    Some(doc! {
                        "author": author
                    })
                } else {
                    None
                })? {
                    println!("\n{}", quote?);
                }
            }
        },
        None => {}
    }

    if let Some(quote_content) = args.quote {
        let mut new_quote = Quote::default();

        new_quote.quote = quote_content;
        if let Some(author) = args.author {
            new_quote.author = Some(author);
        }

        if args.timestamp {
            new_quote.date = Some(chrono::Local::now());
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
