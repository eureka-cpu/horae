use axum::extract::FromRequestParts;
use axum::http::{StatusCode, request::Parts};
use uuid::Uuid;

/// Extractor that authenticates a Harvest API request.
///
/// For MVP this reads the user from the tower-sessions cookie.
/// A future iteration will add `Authorization: Bearer <token>` support
/// with an `api_tokens` table.
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the session from request extensions (set by SessionManagerLayer).
        let session = parts
            .extensions
            .get::<tower_sessions::Session>()
            .cloned()
            .ok_or((StatusCode::UNAUTHORIZED, "No session"))?;

        let user_id = crate::auth::session::get_session_user_id(&session)
            .await
            .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated"))?;

        // Look up the user's org.
        let state = crate::state::global_state().await;
        let row: (Uuid,) = sqlx::query_as("SELECT org_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "User not found"))?;

        Ok(AuthUser {
            user_id,
            org_id: row.0,
        })
    }
}
