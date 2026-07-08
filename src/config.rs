use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_level: String,
    pub data_dir: String,
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
                .unwrap_or_else(|_| "sqlite:horae.db".into()),
            log_level: std::env::var("HORAE_LOG").unwrap_or_else(|_| "info".into()),
            data_dir: std::env::var("HORAE_DATA_DIR").unwrap_or_else(|_| ".".into()),
        })
    }
}
