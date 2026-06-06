use chrono;
use connections_core::Archive;
use maud::{Markup, html};
use std::path::Path as StdPath;

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
        div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.5rem;" {
            (children)
        }
    }
}

fn word_box(word: &str, selected: bool) -> Markup {
    html! {
        button.word.selected[selected] {
            (word)
        }
    }
}

pub async fn game(id_or_date: Option<String>) -> Markup {
    let archive = Archive::load(Some(&StdPath::new("../../archive.json")))
        .await
        .expect("failed to load archive");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let puzzle = archive.get(&id_or_date.unwrap_or(today));
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
        div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%;" {
            h1 { (title) }
            (word_grid(html! {
                @for word in &words {
                    (word_box(word, false))
                }
            }))
        }
    }
}
