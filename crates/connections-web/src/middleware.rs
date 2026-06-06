use axum::{extract::{Request, State}, middleware::Next, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;

use crate::state::AppState;

/// Injected into request extensions so handlers can read the session id.
#[derive(Clone, Debug)]
pub struct SessionId(pub String);

/// Reads the `session_id` cookie; creates a new session row if absent.
/// Touches `last_seen` on every request.
/// Must be applied with `axum::middleware::from_fn_with_state`.
pub async fn session_middleware(
    State(state): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let (session_id, jar) = match jar.get("session_id") {
        Some(cookie) => {
            let id = cookie.value().to_string();
            sqlx::query("UPDATE sessions SET last_seen = datetime('now') WHERE id = ?")
                .bind(&id)
                .execute(&state.db)
                .await
                .ok();
            (id, jar)
        }
        None => {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO sessions (id, created_at, last_seen)
                 VALUES (?, datetime('now'), datetime('now'))",
            )
            .bind(&id)
            .execute(&state.db)
            .await
            .unwrap();

            let cookie = Cookie::build(("session_id", id.clone()))
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .build();

            (id, jar.add(cookie))
        }
    };

    req.extensions_mut().insert(SessionId(session_id));
    let response = next.run(req).await;
    (jar, response)
}
