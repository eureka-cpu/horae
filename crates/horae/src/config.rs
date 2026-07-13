use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_level: String,
    /// When `DEV_LOGIN=1`, skip OIDC and log in as the seeded admin user.
    pub dev_login: bool,
    /// Secret for signing session cookies (set `SESSION_SECRET` in prod).
    pub session_secret: String,
    /// Directory containing plugin subdirectories (each with plugin.toml + *.wasm).
    pub plugins_dir: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: std::env::var("HORAE_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("HORAE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/horae".into()),
            log_level: std::env::var("HORAE_LOG").unwrap_or_else(|_| "info".into()),
            dev_login: std::env::var("DEV_LOGIN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            session_secret: std::env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-me-in-production".into()),
            plugins_dir: std::env::var("HORAE_PLUGINS_DIR")
                .unwrap_or_else(|_| "plugins".into()),
        })
    }
}
