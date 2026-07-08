use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::OnceCell;

use crate::plugin::registry::PluginRegistry;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub plugins: Arc<PluginRegistry>,
}

impl AppState {
    pub fn new(db: SqlitePool, plugins: Arc<PluginRegistry>) -> Self {
        Self { db, plugins }
    }
}

// Async-aware singleton: initialised exactly once, inside dioxus's tokio runtime.
static GLOBAL_STATE: OnceCell<AppState> = OnceCell::const_new();

/// Returns a reference to the global AppState, initialising it on first call.
/// All database work happens inside the caller's tokio runtime, so the pool
/// and pool tasks are always on the same runtime.
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

            let plugins = Arc::new(PluginRegistry::new());
            let plugins_dir = std::path::Path::new(&cfg.data_dir).join("plugins");
            plugins
                .load_from_dir(&plugins_dir)
                .await
                .expect("Failed to scan plugins directory");

            AppState::new(pool, plugins)
        })
        .await
}
