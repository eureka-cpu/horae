use std::fmt;

use thiserror::Error;

#[derive(Debug)]
pub struct AppError(AppErrorKind);

#[derive(Debug, Error)]
enum AppErrorKind {
    #[cfg(feature = "server")]
    #[error("Database error: {0}")]
    Database(sqlx::Error),

    #[error("Authentication failed")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error(transparent)]
    Other(anyhow::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl AppError {
    pub fn unauthorized() -> Self {
        Self(AppErrorKind::Unauthorized)
    }

    pub fn forbidden() -> Self {
        Self(AppErrorKind::Forbidden)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self(AppErrorKind::NotFound(msg.into()))
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self(AppErrorKind::Validation(msg.into()))
    }
}

#[cfg(feature = "server")]
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self(AppErrorKind::Database(e))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self(AppErrorKind::Other(e))
    }
}

impl From<AppError> for dioxus::prelude::ServerFnError {
    fn from(e: AppError) -> Self {
        dioxus::prelude::ServerFnError::ServerError {
            message: e.to_string(),
            code: 500,
            details: None,
        }
    }
}
