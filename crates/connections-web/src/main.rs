use axum::{
    Router,
    extract::{Path, State},
    routing::{delete, get, put},
};
use connections_core::archive::Archive;
use connections_web::{deselect_word, game, select_word};
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path as StdPath;
use std::sync::Arc;

mod middleware;
mod state;

use state::AppState;

async fn human_play_page(
    State(state): State<AppState>,
    Path(id_or_date): Path<String>,
) -> maud::Markup {
    game(state.archive, Some(id_or_date)).await
}

async fn home_page(State(state): State<AppState>) -> maud::Markup {
    game(state.archive, None).await
}

#[tokio::main]
async fn main() {
    let archive = Archive::load(Some(StdPath::new("../../archive.json")))
        .await
        .expect("failed to load archive.json");
    let archive = Arc::new(archive);

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://../../games.db?mode=rwc".to_string());

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("failed to open database");

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("migrations failed");

    let state = AppState { archive, db };

    // Routes that get session middleware applied.
    // New game/api routes go here; the middleware injects SessionId into request extensions.
    let session_routes = Router::new()
        .route("/{id_or_date}", get(human_play_page))
        .route(
            "/api/games/nyt/{id_or_date}/state/words/{word}",
            put(select_word),
        )
        .route(
            "/api/games/nyt/{id_or_date}/state/words/{word}",
            delete(deselect_word),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            self::middleware::session_middleware,
        ));

    let app = Router::new()
        .route("/favicon.ico", get(|| async { "" }))
        .route("/", get(home_page))
        .merge(session_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
