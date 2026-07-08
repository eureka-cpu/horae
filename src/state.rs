use sqlx::PgPool;
use tokio::sync::OnceCell;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

// Async-aware singleton: initialised exactly once, inside dioxus's tokio runtime.
static GLOBAL_STATE: OnceCell<AppState> = OnceCell::const_new();

/// Pre-initialise the global state with an already-created pool.
/// Call this in `main` before starting the Axum server so that session and
/// auth handlers share the same pool as server functions.
pub async fn init_state(pool: sqlx::PgPool) {
    GLOBAL_STATE
        .get_or_init(|| async { AppState::new(pool) })
        .await;
}

/// Returns a reference to the global AppState.
/// Falls back to lazy initialisation if `init_state` was not called (e.g. in tests).
pub async fn global_state() -> &'static AppState {
    GLOBAL_STATE
        .get_or_init(|| async {
            use crate::config::AppConfig;
            use crate::db;

            let cfg = AppConfig::from_env().expect("Failed to read config from env");

            let pool = db::create_pool(&cfg.database_url)
                .await
                .expect("Failed to connect to database");

            db::run_migrations(&pool)
                .await
                .expect("Failed to run migrations");

            AppState::new(pool)
        })
        .await
}
