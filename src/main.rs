use chrono::{Duration, Local, NaiveDate};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "connections", about = "NYT Connections puzzle fetcher")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print all 16 words sorted by position
    Words {
        /// Date YYYY-MM-DD, or "today" (default)
        date: Option<String>,
    },
    /// Print raw JSON for a single puzzle
    Json {
        /// Date YYYY-MM-DD, or "today" (default)
        date: Option<String>,
    },
    /// Fetch all puzzles from today backwards, appending to archive file
    Archive {
        /// Output file (default: archive.json)
        #[arg(short, long, default_value = "archive.json")]
        output: PathBuf,
        /// Earliest date to fetch back to (default: 2023-06-12, first puzzle)
        #[arg(short, long, default_value = "2023-06-12")]
        since: String,
    },
}

#[derive(Deserialize, Serialize, Clone)]
struct Puzzle {
    id: u32,
    print_date: String,
    editor: String,
    categories: Vec<Category>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Category {
    title: String,
    cards: Vec<Card>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Card {
    content: String,
    position: u8,
}

const API: &str = "https://www.nytimes.com/svc/connections/v2";

fn resolve_date(input: Option<&str>) -> String {
    match input.unwrap_or("today") {
        "today" => Local::now().format("%Y-%m-%d").to_string(),
        d => {
            if NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
                eprintln!("Invalid date: {d}. Use YYYY-MM-DD or 'today'.");
                std::process::exit(1);
            }
            d.to_string()
        }
    }
}

fn fetch_puzzle(date: &str) -> Result<Puzzle, String> {
    let url = format!("{API}/{date}.json");
    let resp = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Puzzle>().map_err(|e| e.to_string())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Words { date } => {
            let date = resolve_date(date.as_deref());
            let puzzle = fetch_puzzle(&date).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            let mut all_cards: Vec<(&str, u8)> = puzzle
                .categories
                .iter()
                .flat_map(|c| c.cards.iter().map(|card| (card.content.as_str(), card.position)))
                .collect();
            all_cards.sort_by_key(|(_, pos)| *pos);

            println!("NYT Connections #{} — {}", puzzle.id, puzzle.print_date);
            for (word, pos) in &all_cards {
                println!("{:>2}. {}", pos, word);
            }
        }

        Command::Json { date } => {
            let date = resolve_date(date.as_deref());
            let url = format!("{API}/{date}.json");
            let resp = reqwest::blocking::get(&url).unwrap_or_else(|e| {
                eprintln!("Request failed: {e}");
                std::process::exit(1);
            });
            if !resp.status().is_success() {
                eprintln!("HTTP {}: no puzzle for {date}", resp.status());
                std::process::exit(1);
            }
            println!("{}", resp.text().unwrap());
        }

        Command::Archive { output, since } => {
            let since_date = NaiveDate::parse_from_str(&since, "%Y-%m-%d").unwrap_or_else(|_| {
                eprintln!("Invalid --since date: {since}");
                std::process::exit(1);
            });

            // Load existing archive, build set of already-fetched dates
            let mut archive: Vec<Puzzle> = if output.exists() {
                let text = fs::read_to_string(&output).unwrap_or_default();
                serde_json::from_str(&text).unwrap_or_default()
            } else {
                vec![]
            };

            let cached: HashSet<String> = archive.iter().map(|p| p.print_date.clone()).collect();
            eprintln!("Cached: {} puzzles", cached.len());

            let today = Local::now().date_naive();
            let mut current = today;
            let mut fetched = 0;
            let mut skipped = 0;

            while current >= since_date {
                let date_str = current.format("%Y-%m-%d").to_string();

                if cached.contains(&date_str) {
                    skipped += 1;
                    current -= Duration::days(1);
                    continue;
                }

                match fetch_puzzle(&date_str) {
                    Ok(puzzle) => {
                        eprintln!("Fetched #{} — {}", puzzle.id, date_str);
                        archive.push(puzzle);
                        fetched += 1;
                    }
                    Err(e) => {
                        eprintln!("Skip {date_str}: {e}");
                    }
                }

                current -= Duration::days(1);
                // Polite delay to avoid hammering the API
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            // Sort archive by date descending
            archive.sort_by(|a, b| b.print_date.cmp(&a.print_date));

            let json = serde_json::to_string_pretty(&archive).unwrap();
            fs::write(&output, json).unwrap_or_else(|e| {
                eprintln!("Write failed: {e}");
                std::process::exit(1);
            });

            eprintln!("Done. Fetched {fetched} new, skipped {skipped} cached. Total: {} puzzles → {}", archive.len(), output.display());
        }
    }
}

