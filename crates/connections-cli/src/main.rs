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
    /// Fetch all community puzzles by a username from connectionsplus.io
    UserArchive {
        /// Username to archive (output saved to <username>.json)
        username: String,
        /// Output directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

/// Stored format — `date` is set from the request URL, not the API `print_date` field.
#[derive(Deserialize, Serialize, Clone)]
struct Puzzle {
    #[serde(default)]
    date: String,
    id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    editor: Option<String>,
    categories: Vec<Category>,
}

/// Raw API response shape — `print_date` verified to always equal the request date.
#[derive(Deserialize)]
struct ApiPuzzle {
    id: u32,
    #[serde(default)]
    editor: Option<String>,
    categories: Vec<Category>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Category {
    title: String,
    cards: Vec<Card>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Card {
    /// Text puzzles use `content`; image puzzles use `image_alt_text` instead.
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    image_url: Option<String>,
    #[serde(default)]
    image_alt_text: Option<String>,
    position: u8,
}

impl Card {
    fn label(&self) -> &str {
        self.content
            .as_deref()
            .or(self.image_alt_text.as_deref())
            .unwrap_or("?")
    }
}

const API: &str = "https://www.nytimes.com/svc/connections/v2";
const COMMUNITY_API: &str =
    "https://qybg0x3528.execute-api.us-east-2.amazonaws.com/default/getCommunityGamesV2";

/// Community puzzle from connectionsplus.io list API.
///
/// `categories` is None until we can decrypt game data (connectionsplus.io encrypts puzzle
/// content client-side). Once decryption is solved, populate it to match the NYT format:
///
///   "categories": [
///     {
///       "title": "ASSOCIATED WITH HANSEL AND GRETEL",
///       "cards": [
///         { "content": "WITCH", "position": 0 },
///         ...
///       ]
///     },
///     ...
///   ]
///
/// That will let community games be processed by the same `words`/`json` paths as NYT puzzles.
#[derive(Deserialize, Serialize, Clone)]
struct CommunityGame {
    name: String,
    #[serde(rename = "createdBy")]
    created_by: String,
    id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "attemptedPlays")]
    attempted_plays: u32,
    /// Populated once decryption is implemented; None in the meantime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    categories: Option<Vec<Category>>,
}

#[derive(Deserialize)]
struct CommunityGamesResponse {
    games: Vec<CommunityGame>,
}

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
    let raw = resp.json::<ApiPuzzle>().map_err(|e| e.to_string())?;
    Ok(Puzzle {
        date: date.to_string(),
        id: raw.id,
        editor: raw.editor,
        categories: raw.categories,
    })
}

fn cmd_words(date: Option<String>) {
    let date = resolve_date(date.as_deref());
    let puzzle = fetch_puzzle(&date).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    let mut cards: Vec<(&str, u8)> = puzzle
        .categories
        .iter()
        .flat_map(|c| c.cards.iter().map(|card| (card.label(), card.position)))
        .collect();
    cards.sort_by_key(|(_, pos)| *pos);

    println!("NYT Connections #{} — {}", puzzle.id, puzzle.date);
    for (word, pos) in &cards {
        println!("{:>2}. {word}", pos);
    }
}

fn cmd_json(date: Option<String>) {
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

fn cmd_archive(output: PathBuf, since: String) {
    let since_date = NaiveDate::parse_from_str(&since, "%Y-%m-%d").unwrap_or_else(|_| {
        eprintln!("Invalid --since date: {since}");
        std::process::exit(1);
    });

    let mut archive: Vec<Puzzle> = if output.exists() {
        let text = fs::read_to_string(&output).unwrap_or_default();
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        vec![]
    };

    let cached: HashSet<String> = archive.iter().map(|p| p.date.clone()).collect();
    eprintln!("Cached: {} puzzles", cached.len());

    let mut current = Local::now().date_naive();
    let mut fetched = 0;
    let mut skipped = 0;

    while current >= since_date {
        let date_str = current.format("%Y-%m-%d").to_string();
        current -= Duration::days(1);

        if cached.contains(&date_str) {
            skipped += 1;
            continue;
        }

        match fetch_puzzle(&date_str) {
            Ok(puzzle) => {
                eprintln!("Fetched #{} — {}", puzzle.id, puzzle.date);
                archive.push(puzzle);
                fetched += 1;
            }
            Err(e) => eprintln!("Skip {date_str}: {e}"),
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    archive.sort_by(|a, b| b.date.cmp(&a.date));

    let json = serde_json::to_string_pretty(&archive).unwrap();
    fs::write(&output, json).unwrap_or_else(|e| {
        eprintln!("Write failed: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Done. Fetched {fetched} new, skipped {skipped} cached. Total: {} puzzles → {}",
        archive.len(),
        output.display()
    );
}

/// Placeholder: fetch and decrypt puzzle content for a community game.
/// connectionsplus.io encrypts categories client-side (PBKDF2 + AES-CBC).
/// Once the key derivation is reversed, this should return a Vec<Category>
/// in the same format as NYT puzzles so existing display/archive logic can be reused.
#[allow(unused_variables)]
fn fetch_community_categories(_game_id: &str) -> Option<Vec<Category>> {
    // TODO: implement decryption
    None
}

fn cmd_user_archive(username: String, dir: PathBuf) {
    let output = dir.join(format!("{username}.json"));

    let mut archive: Vec<CommunityGame> = if output.exists() {
        let text = fs::read_to_string(&output).unwrap_or_default();
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        vec![]
    };

    let cached: HashSet<String> = archive.iter().map(|g| g.id.clone()).collect();
    eprintln!("Cached: {} games", cached.len());

    let url = format!("{COMMUNITY_API}?page=1&pageSize=100000&sort=popular&q={username}");
    let resp = reqwest::blocking::get(&url).unwrap_or_else(|e| {
        eprintln!("Request failed: {e}");
        std::process::exit(1);
    });
    if !resp.status().is_success() {
        eprintln!("HTTP {}: fetch failed for user {username}", resp.status());
        std::process::exit(1);
    }
    let body: CommunityGamesResponse = resp.json().unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    });

    let mut fetched = 0;
    let mut skipped = 0;

    for mut game in body.games.into_iter().filter(|g| g.created_by == username) {
        if cached.contains(&game.id) {
            skipped += 1;
            continue;
        }
        game.categories = fetch_community_categories(&game.id);
        eprintln!("Fetched \"{}\" ({})", game.name, game.id);
        archive.push(game);
        fetched += 1;
    }

    archive.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let json = serde_json::to_string_pretty(&archive).unwrap();
    fs::write(&output, json).unwrap_or_else(|e| {
        eprintln!("Write failed: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Done. Fetched {fetched} new, skipped {skipped} cached. Total: {} games → {}",
        archive.len(),
        output.display()
    );
}

fn main() {
    match Cli::parse().command {
        Command::Words { date } => cmd_words(date),
        Command::Json { date } => cmd_json(date),
        Command::Archive { output, since } => cmd_archive(output, since),
        Command::UserArchive { username, dir } => cmd_user_archive(username, dir),
    }
}

