use axum::extract::Path;
use chrono;
use connections_core::archive::SharedArchive;
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
        #word-grid style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.5rem;" {
            (children)
        }
    }
}

fn word_box(word: &str, selected: bool, game_id_or_date: &str) -> Markup {
    let state_path = vec![
        "api/games/nyt/",
        game_id_or_date,
        "/state/words/",
        &word.to_lowercase(),
    ]
    .join("");
    html! {
        @if selected {
            button.word.selected hx-delete=(state_path) hx-swap="outerHTML" {
                (word)
            }
        } @else {
            button.word hx-put=(state_path) hx-swap="outerHTML" {
                (word)
            }
        }
    }
}

pub async fn game(archive: SharedArchive, id_or_date: Option<String>) -> Markup {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let id_or_date = id_or_date.unwrap_or(today);
    let puzzle = archive.get(&id_or_date);
    if puzzle.is_none() {
        return html! { h1 { "Puzzle not found!" } };
    }
    let puzzle = puzzle.expect("puzzle is none even though we checked above");
    let title = puzzle.date.to_string();
    let cards = puzzle
        .categories
        .iter()
        .flat_map(|c| c.cards.iter())
        .collect::<Vec<_>>();
    let words = cards
        .iter()
        .map(|c| c.content.as_deref().unwrap_or_default())
        .collect::<Vec<_>>();

    html! {
        (DOCTYPE)
        meta charset="utf-8";
        meta name="viewport" content="width=device-width, initial-scale=1";
        style {
            "
            .word {
                background: #E5E4E2;
                color: black;
                border-radius: 5px;
                width: 9.375rem;
                min-width: 9.375rem;
                height: 5rem;
                display: flex;
                align-items: center;
                justify-content: center;
                text-align: center;
                user-select: none;
                cursor: pointer;
                border: none;
                font-size: 1.125rem;
                font-weight: 600;
                text-transform: uppercase;
            }
            .word:hover {
                background: #D7D7D7;
            }
            .word.selected {
                background: #555555;
                color: white;
            }
            "
        }
        script src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
            integrity="sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V"
            crossorigin="anonymous";
        div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%;" {
            h1 { (title) }
            (word_grid(html! {
                @for word in &words {
                    (word_box(word, false, &id_or_date))
                }
            }))
        }
    }
}

pub async fn select_word(Path((game_id_or_date, word)): Path<(String, String)>) -> Markup {
    word_box(&word, true, &game_id_or_date)
}

pub async fn deselect_word(Path((game_id_or_date, word)): Path<(String, String)>) -> Markup {
    word_box(&word, false, &game_id_or_date)
}
