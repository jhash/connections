use axum::{
    Router,
    extract::Path,
    routing::{delete, get, put},
};
use maud::Markup;

use connections_web::{deselect_word, game, select_word};

async fn human_play_page(Path(id_or_date): Path<String>) -> Markup {
    game(Some(id_or_date)).await
}

async fn home_page() -> Markup {
    game(None).await
}

#[tokio::main]
async fn main() {
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
        );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
