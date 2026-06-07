use chrono::{Duration, Local, NaiveDate};
use clap::{Parser, Subcommand};
use connections_core::{
    archive::{Archive, ArchiveError, CommunityArchive},
    puzzle::{Category, CommunityGame, NytPuzzle, PuzzleSource},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

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
        /// Date YYYY-MM-DD, puzzle id, or "today" (default)
        date: Option<String>,
        /// Load from <username>.json instead of archive.json (offline lookup)
        #[arg(short, long)]
        user: Option<String>,
    },
    /// Print raw JSON for a single puzzle
    Json {
        /// Date YYYY-MM-DD, or "today" (default)
        date: Option<String>,
    },
    /// Fetch all puzzles from today backwards, appending to archive file
    Archive {
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
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
    /// Seed the SQLite database from archive.json and optional community files
    Seed {
        /// SQLite database file (created if absent)
        #[arg(short, long, default_value = "games.db")]
        db: PathBuf,
        /// NYT archive file
        #[arg(short, long, default_value = "archive.json")]
        archive: PathBuf,
        /// Community username archives to include (e.g. --users chloetron jaycub)
        #[arg(short, long, num_args = 0..)]
        users: Vec<String>,
        /// Directory containing <username>.json files (default: current dir)
        #[arg(long, default_value = ".")]
        users_dir: PathBuf,
    },
}

const NYT_API: &str = "https://www.nytimes.com/svc/connections/v2";
const COMMUNITY_API: &str =
    "https://qybg0x3528.execute-api.us-east-2.amazonaws.com/default/getCommunityGamesV2";

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

fn fetch_puzzle_http(date: &str) -> Result<connections_core::puzzle::Puzzle, String> {
    let url = format!("{NYT_API}/{date}.json");
    let resp = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let raw = resp.json::<NytPuzzle>().map_err(|e| e.to_string())?;
    Ok(connections_core::puzzle::Puzzle {
        date: date.to_string(),
        id: raw.id,
        editor: raw.editor,
        categories: raw.categories,
    })
}

async fn cmd_words(date: Option<String>, user: Option<String>) {
    // If a username is given, look up offline from the local archive file.
    // Otherwise fall back to the NYT API.
    let key = date.as_deref().unwrap_or("today");

    let _puzzle = if let Some(ref username) = user {
        // Community archive lookup — note: categories may be None until decryption is solved.
        let archive = CommunityArchive::load_for_user(username, &PathBuf::from("."))
            .await
            .unwrap_or_else(|e| {
                eprintln!("Error loading {username}.json: {e}");
                std::process::exit(1);
            });
        let game = archive.get(key).unwrap_or_else(|| {
            eprintln!("Game '{key}' not found in {username}.json");
            std::process::exit(1);
        });
        println!(
            "connections+ \"{}\" by {} ({})",
            game.name, game.created_by, game.id
        );
        if game.categories.is_none() {
            eprintln!("Note: categories not yet available (decryption not implemented)");
        }
        return;
    } else if key != "today" {
        // Try loading from local archive first (fast, offline).
        match Archive::load_for_user(None, &PathBuf::from(".")).await {
            Ok(archive) => {
                if let Some(p) = archive.get(key) {
                    print_puzzle_words(p);
                    return;
                }
                // Not in local archive — fall through to API fetch.
            }
            Err(ArchiveError::NotFound(_)) => {} // no local archive, use API
            Err(e) => eprintln!("Warning: could not load local archive: {e}"),
        }
    };

    // Live fetch from NYT API.
    let date = resolve_date(Some(key));
    let puzzle = fetch_puzzle_http(&date).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });
    print_puzzle_words(&puzzle);
}

fn print_puzzle_words(puzzle: &connections_core::puzzle::Puzzle) {
    let mut cards: Vec<(&str, u8)> = puzzle
        .categories
        .iter()
        .flat_map(|c| c.cards.iter().map(|card| (card.label(), card.position)))
        .collect();
    cards.sort_by_key(|(_, pos)| *pos);

    println!("NYT Connections #{} — {}", puzzle.id.unwrap(), puzzle.date);
    for (word, pos) in &cards {
        println!("{:>2}. {word}", pos);
    }
}

fn cmd_json(date: Option<String>) {
    let date = resolve_date(date.as_deref());
    let url = format!("{NYT_API}/{date}.json");
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

async fn cmd_archive(output: PathBuf, since: String) {
    let since_date = NaiveDate::parse_from_str(&since, "%Y-%m-%d").unwrap_or_else(|_| {
        eprintln!("Invalid --since date: {since}");
        std::process::exit(1);
    });

    let mut puzzles: Vec<connections_core::puzzle::Puzzle> =
        match Archive::load(Some(&output)).await {
            Ok(a) => a.all().to_vec(),
            Err(ArchiveError::NotFound(_)) => vec![],
            Err(e) => {
                eprintln!("Error reading archive: {e}");
                std::process::exit(1);
            }
        };

    let cached: HashSet<String> = puzzles.iter().map(|p| p.date.clone()).collect();
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

        match fetch_puzzle_http(&date_str) {
            Ok(puzzle) => {
                eprintln!("Fetched #{} — {}", puzzle.id.unwrap(), puzzle.date);
                puzzles.push(puzzle);
                fetched += 1;
            }
            Err(e) => eprintln!("Skip {date_str}: {e}"),
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    puzzles.sort_by(|a, b| b.date.cmp(&a.date));

    let json = serde_json::to_string_pretty(&puzzles).unwrap();
    fs::write(&output, json).await.unwrap_or_else(|e| {
        eprintln!("Write failed: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Done. Fetched {fetched} new, skipped {skipped} cached. Total: {} puzzles → {}",
        puzzles.len(),
        output.display()
    );
}

/// Placeholder: fetch and decrypt puzzle content for a community game.
/// connectionsplus.io encrypts categories client-side (PBKDF2 + AES-CBC).
/// Once the key derivation is reversed, return Vec<Category> matching NYT format.
#[allow(unused_variables)]
fn fetch_community_categories(_game_id: &str) -> Option<Vec<Category>> {
    // TODO: implement decryption
    None
}

async fn cmd_user_archive(username: String, dir: PathBuf) {
    let output = dir.join(format!("{username}.json"));

    let mut games: Vec<CommunityGame> = match CommunityArchive::load(&output).await {
        Ok(a) => a.all().to_vec(),
        Err(ArchiveError::NotFound(_)) => vec![],
        Err(e) => {
            eprintln!("Error reading {}: {e}", output.display());
            std::process::exit(1);
        }
    };

    let cached: HashSet<String> = games.iter().map(|g| g.id.clone()).collect();
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
        games.push(game);
        fetched += 1;
    }

    games.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let json = serde_json::to_string_pretty(&games).unwrap();
    fs::write(&output, json).await.unwrap_or_else(|e| {
        eprintln!("Write failed: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Done. Fetched {fetched} new, skipped {skipped} cached. Total: {} games → {}",
        games.len(),
        output.display()
    );
}

async fn seed_nyt(pool: &SqlitePool, archive_path: &PathBuf) {
    let archive = Archive::load(Some(archive_path.as_path()))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error loading {}: {e}", archive_path.display());
            std::process::exit(1);
        });

    let source = PuzzleSource::Nytimes.to_string();
    let mut inserted = 0u32;
    let mut skipped = 0u32;

    for puzzle in archive.all() {
        let ext_id = puzzle.id.unwrap().to_string();

        // Insert puzzle row; skip if already present.
        let result = sqlx::query(
            "INSERT OR IGNORE INTO puzzles (source, external_id, author, date, name)
             VALUES (?, ?, ?, ?, NULL)",
        )
        .bind(&source)
        .bind(&ext_id)
        .bind(&puzzle.editor)
        .bind(&puzzle.date)
        .execute(pool)
        .await
        .unwrap();

        if result.rows_affected() == 0 {
            skipped += 1;
            continue; // puzzle + its categories/cards already seeded
        }

        let puzzle_id: i64 =
            sqlx::query_scalar("SELECT id FROM puzzles WHERE source = ? AND external_id = ?")
                .bind(&source)
                .bind(&ext_id)
                .fetch_one(pool)
                .await
                .unwrap();

        for (cat_pos, category) in puzzle.categories.iter().enumerate() {
            let cat_id: i64 = sqlx::query_scalar(
                "INSERT INTO categories (puzzle_id, title, position) VALUES (?, ?, ?) RETURNING id",
            )
            .bind(puzzle_id)
            .bind(&category.title)
            .bind(cat_pos as i64)
            .fetch_one(pool)
            .await
            .unwrap();

            for card in &category.cards {
                sqlx::query(
                    "INSERT INTO cards (category_id, content, image_url, image_alt, position)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(cat_id)
                .bind(&card.content)
                .bind(&card.image_url)
                .bind(&card.image_alt_text)
                .bind(card.position as i64)
                .execute(pool)
                .await
                .unwrap();
            }
        }

        inserted += 1;
    }

    eprintln!(
        "NYT: inserted {inserted} new puzzles, skipped {skipped} already present. Total in archive: {}",
        archive.len()
    );
}

async fn seed_community(pool: &SqlitePool, username: &str, users_dir: &PathBuf) {
    let source = PuzzleSource::ConnectionsPlus.to_string();

    let community = match CommunityArchive::load_for_user(username, users_dir).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Skipping {username}: {e}");
            return;
        }
    };

    let mut inserted = 0u32;
    let mut skipped = 0u32;

    for game in community.all() {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO puzzles (source, external_id, author, date, name)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&source)
        .bind(&game.id)
        .bind(&game.created_by)
        .bind(&game.created_at)
        .bind(&game.name)
        .execute(pool)
        .await
        .unwrap();

        if result.rows_affected() == 0 {
            skipped += 1;
            continue;
        }

        // Categories are not yet available (encrypted). They'll be inserted
        // once decryption is implemented — for now just the puzzle row is enough
        // to reference from game_states.

        inserted += 1;
    }

    eprintln!("{username}: inserted {inserted} new games, skipped {skipped} already present.");
}

async fn cmd_seed(db: PathBuf, archive: PathBuf, users: Vec<String>, users_dir: PathBuf) {
    let db_url = format!("sqlite://{}?mode=rwc", db.display());
    let pool = SqlitePool::connect(&db_url).await.unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {e}", db.display());
        std::process::exit(1);
    });

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Migration failed: {e}");
            std::process::exit(1);
        });

    eprintln!("Seeding NYT archive from {} …", archive.display());
    seed_nyt(&pool, &archive).await;

    for username in &users {
        eprintln!("Seeding community archive for {username} …");
        seed_community(&pool, username, &users_dir).await;
    }

    eprintln!("Done. Database: {}", db.display());
}

#[tokio::main]
async fn main() {
    match Cli::parse().command {
        Command::Words { date, user } => cmd_words(date, user).await,
        Command::Json { date } => cmd_json(date),
        Command::Archive { output, since } => cmd_archive(output, since).await,
        Command::UserArchive { username, dir } => cmd_user_archive(username, dir).await,
        Command::Seed {
            db,
            archive,
            users,
            users_dir,
        } => cmd_seed(db, archive, users, users_dir).await,
    }
}
