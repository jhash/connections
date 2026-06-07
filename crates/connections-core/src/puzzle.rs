use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleSource {
    Nytimes,
    ConnectionsPlus,
}

impl fmt::Display for PuzzleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nytimes => write!(f, "nytimes"),
            Self::ConnectionsPlus => write!(f, "connections_plus"),
        }
    }
}

impl FromStr for PuzzleSource {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nytimes" => Ok(Self::Nytimes),
            "connections_plus" => Ok(Self::ConnectionsPlus),
            _ => Err(format!("unknown source: {s}")),
        }
    }
}

/// NYT puzzle as stored in archive.json.
/// `date` is derived from the request URL — verified to always match `print_date`.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Puzzle {
    #[serde(default)]
    pub date: String,
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
    pub categories: Vec<Category>,
}

/// Raw NYT API response shape (not stored; mapped into Puzzle on fetch).
#[derive(Deserialize)]
pub struct NytPuzzle {
    pub id: Option<i64>,
    #[serde(default)]
    pub editor: Option<String>,
    pub categories: Vec<Category>,
    #[serde(default)]
    pub date: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Category {
    pub title: String,
    pub cards: Vec<Card>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Card {
    /// Text puzzles use `content`; image puzzles use `image_alt_text` instead.
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_alt_text: Option<String>,
    pub position: u8,
}

impl Card {
    pub fn label(&self) -> &str {
        self.content
            .as_deref()
            .or(self.image_alt_text.as_deref())
            .unwrap_or("?")
    }
}

/// Community puzzle from connectionsplus.io list API.
///
/// `categories` is None until decryption is implemented (connectionsplus.io
/// encrypts puzzle content client-side with PBKDF2 + AES-CBC). Once solved,
/// populate to match the NYT format so community games interoperate with the
/// same display and eval logic:
///
///   "categories": [
///     { "title": "ASSOCIATED WITH HANSEL AND GRETEL",
///       "cards": [{ "content": "WITCH", "position": 0 }, ...] },
///     ...
///   ]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CommunityGame {
    pub name: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "attemptedPlays")]
    pub attempted_plays: u64,
    /// None until decryption is implemented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<Category>>,
}
