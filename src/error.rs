use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[cfg(feature = "server")]
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
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
