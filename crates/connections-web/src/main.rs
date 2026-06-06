use axum::{
    Router,
    extract::{Path, State},
    routing::{delete, get, put},
};
use connections_core::archive::{Archive, SharedArchive};
use connections_web::{deselect_word, game, select_word};
use maud::Markup;
use std::path::Path as StdPath;
use std::sync::Arc;

async fn human_play_page(
    State(archive): State<SharedArchive>,
    Path(id_or_date): Path<String>,
) -> Markup {
    game(archive, Some(id_or_date)).await
}

async fn home_page(State(archive): State<SharedArchive>) -> Markup {
    game(archive, None).await
}

#[tokio::main]
async fn main() {
    // TODO: make relative to binary?
    let archive = Archive::load(Some(&StdPath::new("../../archive.json")))
        .await
        .expect("failed to load archive");

    let archive: SharedArchive = Arc::new(archive);

    // build our application with a single route
    let app = Router::new()
        .route("/favicon.ico", get(|| async { "Hello, World!" }))
        .route("/", get(home_page))
        .route("/{id_or_date}", get(human_play_page))
        .route(
            "/api/games/nyt/{id_or_date}/state/words/{word}",
            put(select_word),
        )
        .route(
            "/api/games/nyt/{id_or_date}/state/words/{word}",
            delete(deselect_word),
        )
        .with_state(archive);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
