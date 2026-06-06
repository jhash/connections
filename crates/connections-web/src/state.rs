use connections_core::archive::SharedArchive;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub archive: SharedArchive,
    pub db: SqlitePool,
}
