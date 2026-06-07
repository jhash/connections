use crate::AppState;
use axum::extract::{Path, State};
use chrono;
use connections_core::puzzle::{Card, Category, NytPuzzle};
use maud::{DOCTYPE, Markup, html};

// From gemini
#[allow(unused_macros)]
macro_rules! inline_style {
    ($($prop:literal : $val:expr),* $(,)?) => {{
        let mut styles = Vec::new();
        $(
            styles.push(format!("{}: {}", $prop, $val));
        )*
        styles.join("; ")
    }};
}

fn word_grid(children: Markup) -> Markup {
    html! {
        #word-grid.word-grid {
            (children)
        }
    }
}

fn word_box(
    word: &str,
    selected: bool,
    puzzle_id: &i64,
    session_id: &str,
    card_id: &i64,
) -> Markup {
    let update_path = vec![
        "api/puzzles",
        &puzzle_id.to_string(),
        "sessions",
        session_id,
        "selected_cards",
        &card_id.to_string(),
    ]
    .join("/");
    html! {
        @if selected {
            button.word.selected hx-delete=(update_path) hx-swap="outerHTML" {
                (word)
            }
        } @else {
            button.word hx-put=(update_path) hx-swap="outerHTML" {
                (word)
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct SelectedCards(u16);

impl SelectedCards {
    pub fn is_selected(self, position: u8) -> bool {
        (self.0 >> position) & 1 == 1
    }

    pub fn select(&mut self, position: u8) {
        self.0 |= 1 << position;
    }

    pub fn deselect(&mut self, position: u8) {
        self.0 &= !(1 << position);
    }

    pub fn toggle(&mut self, position: u8) {
        self.0 ^= 1 << position;
    }

    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<i64> for SelectedCards {
    fn from(v: i64) -> Self {
        SelectedCards(v as u16)
    }
}

struct GameState {
    pub id: i64,
    pub lives: u8,
    pub selected: SelectedCards,
}

async fn save_selected(state: &AppState, game_state: &GameState) {
    let mask = game_state.selected.as_u16() as i64;
    sqlx::query!(
        "UPDATE game_states SET selected_mask = ? WHERE id = ?",
        mask,
        game_state.id
    )
    .execute(&state.db)
    .await
    .expect("failed to save selected mask");
}

async fn find_or_create_game_state(
    state: &AppState,
    session_id: &str,
    puzzle_id: &i64,
) -> GameState {
    let existing = sqlx::query!(
        "SELECT id, lives, selected_mask FROM game_states
           WHERE session_id = ? AND puzzle_id = ?",
        session_id,
        puzzle_id
    )
    .fetch_optional(&state.db)
    .await
    .expect("failed to fetch game state");

    if let Some(row) = existing {
        return GameState {
            id: row.id.expect("game_state.id is null"),
            lives: row.lives as u8,
            selected: SelectedCards::from(row.selected_mask),
        };
    }

    let id = sqlx::query!(
        "INSERT INTO game_states (session_id, puzzle_id) VALUES (?, ?) RETURNING id",
        session_id,
        puzzle_id
    )
    .fetch_one(&state.db)
    .await
    .expect("failed to create game state")
    .id
    .expect("inserted game_state.id is null");

    GameState {
        id,
        lives: 4,
        selected: SelectedCards::default(),
    }
}

async fn get_card(state: &AppState, id: i64) -> Option<Card> {
    let card = sqlx::query!("SELECT id, content, position FROM cards WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .expect("failed to fetch card");

    if card.is_none() {
        return None;
    }

    let card = card.unwrap();

    Some(Card {
        id: Some(card.id),
        content: card.content,
        position: card.position as u8,
        image_alt_text: None,
        image_url: None,
    })
}

async fn get_puzzle(state: &AppState, date: &str) -> Option<NytPuzzle> {
    let puzzle = sqlx::query!(
        "SELECT id, external_id, author, date, name FROM puzzles
           WHERE source = 'nytimes' AND date LIKE ?",
        date
    )
    .fetch_optional(&state.db)
    .await
    .expect("failed to fetch puzzle");

    if puzzle.is_none() {
        return None;
    }

    let puzzle = puzzle.unwrap();

    let rows = sqlx::query!(
        "SELECT c.id as category_id, c.title, c.position,
                ca.id as card_id, ca.content, ca.image_url, ca.image_alt, ca.position as card_position
         FROM categories c
         LEFT JOIN cards ca ON ca.category_id = c.id
         WHERE c.puzzle_id = ?
         ORDER BY c.position, ca.position",
        puzzle.id
    )
    .fetch_all(&state.db)
    .await
    .expect("failed to fetch puzzle data");

    let mut categories: std::collections::HashMap<i64, Category> = std::collections::HashMap::new();

    for row in rows {
        categories
            // TODO: unwrap bad here?
            .entry(row.category_id.unwrap())
            .or_insert_with(|| Category {
                title: row.title.clone(),
                cards: Vec::new(),
            })
            .cards
            .push(Card {
                id: row.card_id,
                content: row.content,
                image_url: row.image_url,
                image_alt_text: row.image_alt,
                position: row.card_position as u8,
            });
    }

    let categories = categories.into_values().collect::<Vec<_>>();

    Some(NytPuzzle {
        id: puzzle.id,
        editor: puzzle.author,
        categories: categories,
        // TODO: safe to unwrap here?
        date: puzzle.date.unwrap(),
    })
}

fn game_actions(game_state: &GameState, swap: bool) -> Markup {
    let submit_disabled = game_state.selected.count() < 4;
    let deselect_all_disabled = game_state.selected.count() == 0;
    let swap_oob = if swap { "true" } else { "false" };

    html! {
        #game-actions.game-actions hx-swap-oob=(swap_oob) {
            button.game-button disabled[deselect_all_disabled] {
                "Deselect All"
            }
            button.game-button disabled[submit_disabled] {
                "Submit"
            }
        }
    }
}

pub async fn game_page(state: AppState, id_or_date: Option<String>, session_id: String) -> Markup {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let id_or_date = id_or_date.unwrap_or(today);
    // let puzzle = state.archive.get(&id_or_date);
    let puzzle = get_puzzle(&state, &id_or_date).await;
    if puzzle.is_none() {
        return html! { h1 { "Puzzle not found!" } };
    }
    let puzzle = puzzle.expect("puzzle is none even though we checked above");
    let title = puzzle.date.to_string();
    let mut cards = puzzle
        .categories
        .iter()
        .flat_map(|c| c.cards.iter())
        .collect::<Vec<_>>();

    cards.sort_by(|a, b| a.position.cmp(&b.position));

    let game_state = find_or_create_game_state(&state, &session_id, &puzzle.id.unwrap()).await;
    let lives = game_state.lives;
    let actions = game_actions(&game_state, false);

    html! {
        (DOCTYPE)
        meta charset="utf-8";
        meta name="viewport" content="width=device-width, initial-scale=1";
        style {
            "
            html, body {
                width: 100vw;
                max-width: 100vw;
                min-height: 100vh;
                overflow-x: hidden;
                margin: 0;
                padding: 0;
                display: flex;
                flex-direction: column;
            }
            h1, h2, h3, h4, h5, h6 {
                margin: 0;
                padding: 0;
            }
            .game-container {
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                height: 100%;
                max-width: 100%;
                width: 100%;
                flex-grow: 1;
                padding-bottom: 12rem;
                gap: 2rem;
            }
            .word-grid {
                display: grid;
                grid-template-columns: repeat(4, 1fr);
                gap: 0.5rem;
                max-width: 100%;
                padding: 0 0.5rem;
            }
            .word {
                background: #E5E4E2;
                color: black;
                border-radius: 5px;
                width: 9.375rem;
                min-width: 9.375rem;
                height: 5rem;
                align-items: center;
                text-align: center;
                user-select: none;
                cursor: pointer;
                border: none;
                font-size: 1.125rem;
                font-weight: 600;
                text-transform: uppercase;
                @media (max-width: 680px) {
                    width: auto;
                    min-width: auto;
                    max-width: 9.375rem;
                    font-size: 1rem;
                    white-space: break-spaces;
                }
            }
            .word:hover {
                background: #D7D7D7;
            }
            .word.selected {
                background: #555555;
                color: white;
            }
            .game-actions {
                display: flex;
                gap: 0.75rem;
            }
            .game-button {
                background: white;
                border: 1px solid black;
                padding: 0 1rem;
                font-size: 0.875rem;
                font-weight: 600;
                min-width: 5.5rem;
                max-width: 80vw;
                height: 3rem;
                cursor: pointer;
                border-radius: 32px;
                line-height: 1.5em;
            }
            .game-button[disabled] {
                background: #fff;
                color: #8b8b8b;
                border-color: #979797;
            }
            "
        }
        script src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
            integrity="sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V"
            crossorigin="anonymous" {}
        .game-container {
            h1 { (title) }
            h5 { ("Lives: ")(lives) }
            (word_grid(html! {
                @for card in cards {
                    (word_box(&card.content.as_deref().unwrap(), game_state.selected.is_selected(card.position), &puzzle.id.unwrap(), &session_id, &card.id.unwrap()))
                }
            }))
            (actions)
        }
    }
}

// "/api/puzzles/{puzzle_id}/sessions/{session_id}/selected_cards/{card_id}"
pub async fn select_word(
    State(state): State<AppState>,
    Path((puzzle_id, session_id, card_id)): Path<(i64, String, String)>,
) -> Markup {
    let card = get_card(&state, card_id.parse().unwrap()).await;
    if card.is_none() {
        println!("Card is none: {}", card_id)
    }
    let card = card.unwrap();
    let mut game_state = find_or_create_game_state(&state, &session_id, &puzzle_id).await;
    // TODO: need to "deselect" once solved
    if game_state.selected.count() == 4 {
        // TODO: shake
        return word_box(
            &card.content.as_deref().unwrap(),
            false,
            &puzzle_id,
            &session_id,
            &card.id.unwrap(),
        );
    }
    // TODO: multi-select? I don't think NYT supports
    game_state.selected.select(card.position);
    save_selected(&state, &game_state).await;

    let actions = game_actions(&game_state, true);

    html! {
        (actions)
        (word_box(
            &card.content.as_deref().unwrap(),
            true,
            &puzzle_id,
            &session_id,
            &card.id.unwrap(),
        ))
    }
}

pub async fn deselect_word(
    State(state): State<AppState>,
    Path((puzzle_id, session_id, card_id)): Path<(i64, String, String)>,
) -> Markup {
    let card = get_card(&state, card_id.parse().unwrap()).await.unwrap();
    let mut game_state = find_or_create_game_state(&state, &session_id, &puzzle_id).await;
    game_state.selected.deselect(card.position);
    save_selected(&state, &game_state).await;

    let actions = game_actions(&game_state, true);

    html! {
        (actions)
        (word_box(
            &card.content.as_deref().unwrap(),
            false,
            &puzzle_id,
            &session_id,
            &card.id.unwrap(),
        ))
    }
}
