use axum::{
    Router,
    extract::{Path, State},
    routing::{delete, get, put},
};
use connections_core::archive::Archive;
use connections_web::{deselect_word, game, select_word};
use listenfd::ListenFd;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::PathBuf;
use std::sync::Arc;

mod middleware;
mod state;

use state::AppState;

/// Resolve a path relative to the workspace root (two levels up from this crate).
/// Works regardless of the current working directory when the binary is invoked.
fn workspace_path(relative: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is set at compile time to the crate directory.
    // Two levels up from crates/connections-web/ is the repo root.
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir.join("../..").join(relative)
}

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
    let archive_path = std::env::var("ARCHIVE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_path("archive.json"));

    let archive = Archive::load(Some(&archive_path))
        .await
        .unwrap_or_else(|e| {
            eprintln!(
                "Failed to load archive at {}: {e}\nOverride with ARCHIVE_PATH env var.",
                archive_path.display()
            );
            std::process::exit(1);
        });
    let archive = Arc::new(archive);

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}?mode=rwc", workspace_path("games.db").display()));

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to open database ({db_url}): {e}");
            std::process::exit(1);
        });

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("migrations failed");

    let state = AppState { archive, db };

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

    // In dev: systemfd holds the socket across recompiles so the port never drops.
    // Falls back to a fresh bind for production or plain `cargo run`.
    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        Some(std_listener) => {
            // systemfd passes a blocking socket; tokio requires non-blocking.
            std_listener.set_nonblocking(true).unwrap();
            tokio::net::TcpListener::from_std(std_listener).unwrap()
        }
        None => {
            let addr = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
            tokio::net::TcpListener::bind(&addr).await.unwrap()
        }
    };

    println!("Listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
