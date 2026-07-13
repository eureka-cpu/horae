use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::OnceCell;

use crate::plugin::PluginRegistry;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub plugins: Arc<PluginRegistry>,
}

impl AppState {
    pub fn new(db: PgPool, plugins: Arc<PluginRegistry>) -> Self {
        Self { db, plugins }
    }
}

// Async-aware singleton: initialised exactly once, inside dioxus's tokio runtime.
static GLOBAL_STATE: OnceCell<AppState> = OnceCell::const_new();

/// Pre-initialise the global state with an already-created pool and plugin registry.
/// Call this in `main` before starting the Axum server so that session and
/// auth handlers share the same pool as server functions.
pub async fn init_state(pool: sqlx::PgPool, plugins: Arc<PluginRegistry>) {
    GLOBAL_STATE
        .get_or_init(|| async { AppState::new(pool, plugins) })
        .await;
}

/// Returns a reference to the global AppState.
/// Falls back to lazy initialisation if `init_state` was not called (e.g. in tests).
pub async fn global_state() -> &'static AppState {
    GLOBAL_STATE
        .get_or_init(|| async {
            use crate::config::AppConfig;
            use crate::db;

            let cfg = match AppConfig::from_env() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to read config from env: {e}");
                    std::process::exit(1);
                }
            };

            let pool = match db::create_pool(&cfg.database_url).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to connect to database: {e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = db::run_migrations(&pool).await {
                tracing::error!("Failed to run migrations: {e}");
                std::process::exit(1);
            }

            AppState::new(pool, Arc::new(PluginRegistry::empty()))
        })
        .await
}
