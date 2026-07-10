use tower_sessions::Session;
use uuid::Uuid;

pub const SESSION_USER_KEY: &str = "user_id";

/// Returns the authenticated user's UUID from the session, or `None` if not logged in.
pub async fn get_session_user_id(session: &Session) -> Option<Uuid> {
    session.get::<Uuid>(SESSION_USER_KEY).await.ok().flatten()
}

/// Write the authenticated user's UUID into the session.
pub async fn set_session_user_id(session: &Session, user_id: Uuid) -> anyhow::Result<()> {
    session.insert(SESSION_USER_KEY, user_id).await?;
    Ok(())
}

/// Remove the user from the session (logout).
pub async fn clear_session(session: &Session) -> anyhow::Result<()> {
    session.flush().await?;
    Ok(())
}
