use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;

use crate::AppState;

/// Injected into request extensions so handlers can read the session id.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SessionId(pub String);

impl SessionId {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
            let updated = sqlx::query("UPDATE sessions SET last_seen = datetime('now') WHERE id = ?")
                .bind(&id)
                .execute(&state.db)
                .await
                .map(|r| r.rows_affected())
                .unwrap_or(0);

            if updated == 0 {
                // Cookie references a session row that no longer exists (e.g. db was
                // reseeded/reset since the cookie was issued). Recreate it rather than
                // letting downstream inserts fail their FK constraint on session_id.
                sqlx::query(
                    "INSERT INTO sessions (id, created_at, last_seen)
                     VALUES (?, datetime('now'), datetime('now'))",
                )
                .bind(&id)
                .execute(&state.db)
                .await
                .unwrap();
            }
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
