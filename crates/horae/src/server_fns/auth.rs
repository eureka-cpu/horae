//! Auth server functions.

use super::*;

// ── Auth ─────────────────────────────────────────────────────────────────────
// Login is not a server fn — the real flow goes through the Axum `/auth/login`
// route (OIDC / dev-login); see `src/auth/`.

/// Destroy the current session (logout).
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::clear_session(&session)
        .await
        .map_err(server_err)
}

/// Return the currently authenticated user, or 401 if not logged in.
#[server]
pub async fn get_me() -> Result<User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("User not found"))
}
