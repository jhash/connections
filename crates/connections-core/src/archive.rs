use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;

use crate::puzzle::{CommunityGame, Puzzle};

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("file not found: {0}")]
    NotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

/// In-memory index of NYT puzzles loaded from archive.json (or a date-specific file).
///
/// Lookup is O(1) by date string ("YYYY-MM-DD") or numeric puzzle id.
/// Building the HashMap index is done once at load time (~1ms for the full 1,090-puzzle archive).
/// A file-backed index is not warranted at this scale but is straightforward to add if the
/// archive grows significantly after taxonomy annotations are added (Plan 01).
pub struct Archive {
    puzzles: Vec<Puzzle>,
    by_date: HashMap<String, usize>, // date → index into puzzles
    by_id: HashMap<i64, usize>,      // id → index into puzzles
}

impl Archive {
    /// Load an NYT puzzle archive from a JSON file.
    /// Pass `username` to load from `<username>.json` in `dir` instead of the default path.
    pub async fn load(path: Option<&Path>) -> Result<Self, ArchiveError> {
        let path = path.unwrap_or(&Path::new("archive.json"));
        if !path.exists() {
            return Err(ArchiveError::NotFound(path.to_path_buf()));
        }
        let text = fs::read_to_string(path).await?;
        let puzzles: Vec<Puzzle> = serde_json::from_str(&text)?;
        Ok(Self::from_puzzles(puzzles))
    }

    /// Convenience: resolve path from an optional username and base directory.
    ///
    /// - `username = None`  → `{dir}/archive.json`
    /// - `username = Some("chloetron")` → `{dir}/chloetron.json` (community archive)
    ///
    /// Note: community archives store `CommunityGame`, not `Puzzle`. Use
    /// `CommunityArchive::load_for_user` for those.
    pub async fn load_for_user(username: Option<&str>, dir: &Path) -> Result<Self, ArchiveError> {
        let filename = match username {
            None => "archive.json".to_string(),
            Some(u) => format!("{u}.json"),
        };
        Self::load(Some(&dir.join(filename))).await
    }

    fn from_puzzles(puzzles: Vec<Puzzle>) -> Self {
        let mut by_date = HashMap::with_capacity(puzzles.len());
        let mut by_id = HashMap::with_capacity(puzzles.len());
        for (i, p) in puzzles.iter().enumerate() {
            by_date.insert(p.date.clone(), i);
            by_id.insert(p.id.unwrap(), i);
        }
        Self {
            puzzles,
            by_date,
            by_id,
        }
    }

    /// Look up a puzzle by date ("YYYY-MM-DD") or numeric id (as a string, e.g. "512").
    /// Tries date format first; falls back to parsing as i64 id.
    pub fn get(&self, key: &str) -> Option<&Puzzle> {
        // Try as date first (most common case)
        if let Some(&i) = self.by_date.get(key) {
            return Some(&self.puzzles[i]);
        }
        // Fall back to numeric id
        if let Ok(id) = key.parse::<i64>() {
            return self.get_by_id(id);
        }
        None
    }

    pub fn get_by_date(&self, date: &str) -> Option<&Puzzle> {
        self.by_date.get(date).map(|&i| &self.puzzles[i])
    }

    pub fn get_by_id(&self, id: i64) -> Option<&Puzzle> {
        self.by_id.get(&id).map(|&i| &self.puzzles[i])
    }

    pub fn len(&self) -> usize {
        self.puzzles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.puzzles.is_empty()
    }

    /// All puzzles, sorted descending by date (archive.json order).
    pub fn all(&self) -> &[Puzzle] {
        &self.puzzles
    }

    /// Dates present in the archive, for cache-miss checking.
    pub fn dates(&self) -> impl Iterator<Item = &str> {
        self.by_date.keys().map(String::as_str)
    }
}

pub type SharedArchive = Arc<Archive>;

/// In-memory index of community puzzles from a `<username>.json` file.
pub struct CommunityArchive {
    games: Vec<CommunityGame>,
    by_id: HashMap<String, usize>, // game id → index
}

impl CommunityArchive {
    pub async fn load(path: &Path) -> Result<Self, ArchiveError> {
        if !path.exists() {
            return Err(ArchiveError::NotFound(path.to_path_buf()));
        }
        let text = fs::read_to_string(path).await?;
        let games: Vec<CommunityGame> = serde_json::from_str(&text)?;
        Ok(Self::from_games(games))
    }

    pub async fn load_for_user(username: &str, dir: &Path) -> Result<Self, ArchiveError> {
        Self::load(&dir.join(format!("{username}.json"))).await
    }

    fn from_games(games: Vec<CommunityGame>) -> Self {
        let mut by_id = HashMap::with_capacity(games.len());
        for (i, g) in games.iter().enumerate() {
            by_id.insert(g.id.clone(), i);
        }
        Self { games, by_id }
    }

    /// Look up by game id string (e.g. "tBZCr6").
    pub fn get(&self, id: &str) -> Option<&CommunityGame> {
        self.by_id.get(id).map(|&i| &self.games[i])
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    pub fn all(&self) -> &[CommunityGame] {
        &self.games
    }

    /// IDs present, for cache-miss checking.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
    }
}
