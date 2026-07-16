use crate::AppState;
use axum::extract::{Path, State};
use chrono;
use connections_core::puzzle::{Card, Category, NytPuzzle};
use maud::{DOCTYPE, Markup, html};
use std::collections::HashMap;

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
    solved: Option<(u8, &str)>,
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
    let longest_word_len = word.split_whitespace().map(str::len).max().unwrap_or(0);
    let fit_class = match longest_word_len {
        0..=12 => "",
        13..=16 => " word-fit-2",
        _ => " word-fit-3",
    };
    let mut class = format!("word{}", fit_class);
    let mut title = None;
    let mut disabled = false;

    if let Some((cat_pos, cat_title)) = solved {
        class = format!("{} solved-{}", class, cat_pos);
        title = Some(cat_title.to_string());
        disabled = true;
    } else if selected {
        class = format!("{} selected", class);
    }

    html! {
        @if !disabled {
            @if selected {
                button class=(class.as_str()) hx-delete=(update_path) hx-swap="outerHTML" title=(title.unwrap_or_default()) {
                    (word)
                }
            } @else {
                button class=(class.as_str()) hx-put=(update_path) hx-swap="outerHTML" title=(title.unwrap_or_default()) {
                    (word)
                }
            }
        } @else {
            div class=(class.as_str()) {
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
                position: Some(row.position as u8),
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

fn game_actions(game_state: &GameState, swap: bool, puzzle_id: i64, session_id: &str) -> Markup {
    let submit_disabled = game_state.selected.count() < 4;
    let deselect_all_disabled = game_state.selected.count() == 0;
    let swap_oob = if swap { "true" } else { "false" };

    let deselect_all_path = vec![
        "api/puzzles",
        &puzzle_id.to_string(),
        "sessions",
        session_id,
        "selected_cards",
    ]
    .join("/");

    let guess_path = vec![
        "api/puzzles",
        &puzzle_id.to_string(),
        "sessions",
        session_id,
        "guesses",
    ]
    .join("/");

    html! {
        #game-actions.game-actions hx-swap-oob=(swap_oob) {
            button.game-button.disabled[deselect_all_disabled] hx-delete=(deselect_all_path) hx-swap="outerHTML" { "Deselect All" }
            button.game-button.disabled[submit_disabled] hx-post=(guess_path) hx-swap="outerHTML" { "Submit" }
        }
    }
}

async fn game_container(
    state: &AppState,
    game_state: &GameState,
    puzzle: &NytPuzzle,
    session_id: &str,
    swap: bool,
) -> Markup {
    let swap_oob = if swap { "true" } else { "false" };

    let title = puzzle.date.to_string();
    let lives = game_state.lives;

    let mut cards = puzzle
        .categories
        .iter()
        .flat_map(|c| c.cards.iter())
        .collect::<Vec<_>>();

    cards.sort_by(|a, b| a.position.cmp(&b.position));

    // Fetch solved categories for this game state
    let solved_rows = sqlx::query!(
        "SELECT c.position as cat_pos, ca.position as tile_pos, c.title
         FROM solved_categories sc
         JOIN categories c ON sc.category_id = c.id
         JOIN cards ca ON ca.category_id = c.id
         WHERE sc.game_state_id = ?",
        game_state.id
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut solved_map = HashMap::new();
    for row in solved_rows {
        solved_map.insert(row.tile_pos as u8, (row.cat_pos as u8, row.title));
    }

    let game_over = game_state.lives == 0;
    let mut all_solved_map = solved_map.clone();
    if game_over {
        // Add all categories' tiles to solved_map
        for (idx, category) in puzzle.categories.iter().enumerate() {
            let cat_pos = category.position.unwrap_or(idx as u8);
            let title = category.title.clone();
            for card in &category.cards {
                all_solved_map.insert(card.position, (cat_pos, title.clone()));
            }
        }
    }

    let cards = cards
        .into_iter()
        .filter(|card| !solved_map.contains_key(&card.position));

    let actions = game_actions(&game_state, swap, puzzle.id.unwrap(), &session_id);

    html! {
        #game-container.game-container hx-swap-oob=(swap_oob) {
            h1 { (title) }
            h5 { ("Lives: ")(lives) }
            (word_grid(html! {
                @if game_over {
                    div.game-over { "Game Over! All categories revealed." }
                }
                @for card in cards {
                    @let card_pos = card.position;
                    @let solved = all_solved_map.get(&card_pos);
                    (word_box(
                        &card.content.as_deref().unwrap(),
                        game_state.selected.is_selected(card_pos),
                        &puzzle.id.unwrap(),
                        &session_id,
                        &card.id.unwrap(),
                        solved.map(|&(cat_pos, ref title)| (cat_pos, title.as_str()))
                    ))
                }
            }))
            (actions)
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
    let game_state = find_or_create_game_state(&state, &session_id, &puzzle.id.unwrap()).await;

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
                container-type: inline-size;
                background: #efefe6;
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
                font-weight: 600;
                text-transform: uppercase;
                padding: 0.25rem;
                overflow: hidden;
                font-size: clamp(0.65rem, 13cqw, 1.125rem);
                word-break: normal;
                overflow-wrap: normal;
                white-space: normal;
                line-height: 1.15;
            }
            .word-fit-2 {
                font-size: clamp(0.6rem, 11cqw, 0.9rem);
            }
            .word-fit-3 {
                font-size: clamp(0.55rem, 9cqw, 0.75rem);
            }
            @media (max-width: 680px) {
                .word {
                    width: auto;
                    min-width: auto;
                    max-width: 9.375rem;
                }
            }
            .word:hover {
                background: #D7D7D7;
            }
            .word.selected {
                background: #5a594e;
                color: white;
            }
            .solved-0 { background: #f9df6d; }
            .solved-1 { background: #a0c35a; }
            .solved-2 { background: #b0c4ef; }
            .solved-3 { background: #ba81c5; }
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
            @keyframes shake {
                0%, 100% { transform: translateX(0); }
                25% { transform: translateX(-8px); }
                75% { transform: translateX(8px); }
            }
            .shake {
                animation: shake 0.4s ease-in-out;
            }
            "
        }
        script src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
            integrity="sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V"
            crossorigin="anonymous" {}
        (game_container(
            &state,
            &game_state,
            &puzzle,
            &session_id,
            false,
        ).await)
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
            None,
        );
    }
    // TODO: multi-select? I don't think NYT supports
    game_state.selected.select(card.position);
    save_selected(&state, &game_state).await;

    let actions = game_actions(&game_state, true, puzzle_id, &session_id);

    html! {
        (actions)
        (word_box(
            &card.content.as_deref().unwrap(),
            true,
            &puzzle_id,
            &session_id,
            &card.id.unwrap(),
            None,
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

    let actions = game_actions(&game_state, true, puzzle_id, &session_id);

    html! {
        (actions)
        (word_box(
            &card.content.as_deref().unwrap(),
            false,
            &puzzle_id,
            &session_id,
            &card.id.unwrap(),
            None,
        ))
    }
}

pub async fn deselect_all(
    State(state): State<AppState>,
    Path((puzzle_id, session_id)): Path<(i64, String)>,
) -> Markup {
    let mut game_state = find_or_create_game_state(&state, &session_id, &puzzle_id).await;
    game_state.selected.clear();
    save_selected(&state, &game_state).await;

    let actions = game_actions(&game_state, true, puzzle_id, &session_id);

    html! {
        (actions)
    }
}

pub async fn submit_guess(
    State(state): State<AppState>,
    Path((puzzle_id, session_id)): Path<(i64, String)>,
) -> Markup {
    let puzzle = match get_puzzle_by_id(&state, puzzle_id).await {
        Some(puzzle) => puzzle,
        None => return html! { h1 { "Puzzle not found!" } },
    };
    let mut game_state = find_or_create_game_state(&state, &session_id, &puzzle_id).await;

    let selected_mask = game_state.selected.as_u16();
    let mut selected_positions = Vec::new();
    for pos in 0..16 {
        if (selected_mask >> pos) & 1 == 1 {
            selected_positions.push(pos as u8);
        }
    }

    if selected_positions.len() != 4 {
        return game_page(state, Some(puzzle.date.clone()), session_id).await;
    }

    let mut cards_map = HashMap::new();
    for category in &puzzle.categories {
        for card in &category.cards {
            cards_map.insert(card.position, card);
        }
    }

    let mut selected_cards = Vec::new();
    for &pos in &selected_positions {
        if let Some(card) = cards_map.get(&pos) {
            selected_cards.push(card);
        } else {
            return game_page(state, Some(puzzle.date.clone()), session_id).await;
        }
    }

    let mut category_ids: Vec<i64> = selected_cards.iter().filter_map(|card| card.id).collect();
    category_ids.sort();
    let unique_cats = category_ids.windows(2).all(|w| w[0] == w[1]);
    let correct_guess = unique_cats && category_ids.len() == 4;

    if correct_guess {
        let card_id = selected_cards[0].id.expect("Selected card must have an id");
        let category_id = sqlx::query!("SELECT category_id FROM cards WHERE id = ?", card_id)
            .fetch_one(&state.db)
            .await
            .expect("Failed to fetch category_id for card")
            .category_id;

        let turn = sqlx::query!(
            "SELECT COUNT(*) as count FROM guesses WHERE game_state_id = ?",
            game_state.id
        )
        .fetch_one(&state.db)
        .await
        .map(|row| row.count as i32 + 1)
        .unwrap_or(1);

        // TODO: bitmask instead of individual card ids? hard to query for previous guesses

        let c0 = selected_cards[0].id.unwrap();
        let c1 = selected_cards[1].id.unwrap();
        let c2 = selected_cards[2].id.unwrap();
        let c3 = selected_cards[3].id.unwrap();
        sqlx::query!(
            "INSERT INTO guesses (game_state_id, turn, card_id_1, card_id_2, card_id_3, card_id_4, result)
             VALUES (?, ?, ?, ?, ?, ?, 'correct')",
            game_state.id,
            turn,
            c0,
            c1,
            c2,
            c3,
        )
        .execute(&state.db)
        .await
        .expect("failed to insert correct guess");

        sqlx::query!(
            "INSERT INTO solved_categories (game_state_id, category_id, turn)
             VALUES (?, ?, ?)",
            game_state.id,
            category_id,
            turn,
        )
        .execute(&state.db)
        .await
        .expect("failed to insert solved category");

        game_state.selected.clear();
        save_selected(&state, &game_state).await;
    } else {
        if game_state.lives > 0 {
            game_state.lives -= 1;
        }

        let turn = sqlx::query!(
            "SELECT COUNT(*) as count FROM guesses WHERE game_state_id = ?",
            game_state.id
        )
        .fetch_one(&state.db)
        .await
        .map(|row| row.count as i32 + 1)
        .unwrap_or(1);

        // TODO: one_away
        // TODO: bitmask instead of individual card ids? hard to query for previous guesses

        let c0 = selected_cards[0].id.unwrap();
        let c1 = selected_cards[1].id.unwrap();
        let c2 = selected_cards[2].id.unwrap();
        let c3 = selected_cards[3].id.unwrap();
        sqlx::query!(
            "INSERT INTO guesses (game_state_id, turn, card_id_1, card_id_2, card_id_3, card_id_4, result)
             VALUES (?, ?, ?, ?, ?, ?, 'wrong')",
            game_state.id,
            turn,
            c0,
            c1,
            c2,
            c3,
        )
        .execute(&state.db)
        .await
        .expect("failed to insert wrong guess");

        sqlx::query!(
            "UPDATE game_states SET lives = ? WHERE id = ?",
            game_state.lives,
            game_state.id
        )
        .execute(&state.db)
        .await
        .expect("failed to update game state");
    }

    game_container(&state, &game_state, &puzzle, &session_id, true).await
}

async fn get_puzzle_by_id(state: &AppState, puzzle_id: i64) -> Option<NytPuzzle> {
    let puzzle = sqlx::query!(
        "SELECT id, external_id, author, date, name FROM puzzles WHERE id = ? AND source = 'nytimes'",
        puzzle_id
    )
    .fetch_optional(&state.db)
    .await
    .expect("failed to fetch puzzle by id");

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

    let mut categories: HashMap<i64, Category> = HashMap::new();

    for row in rows {
        categories
            .entry(row.category_id.unwrap())
            .or_insert_with(|| Category {
                title: row.title.clone(),
                cards: Vec::new(),
                position: Some(row.position as u8),
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
        id: Some(puzzle.id),
        editor: puzzle.author,
        categories: categories,
        date: puzzle.date.unwrap(),
    })
}
