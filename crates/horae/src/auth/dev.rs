/// DEV_LOGIN handlers — only reachable when DEV_LOGIN=1.
///
/// GET  /auth/login      → HTML page with a one-click "Sign in as Admin" button.
/// POST /auth/dev-login  → sets session to the seeded admin user, redirects to /.
/// POST /auth/logout     → flushes the session, redirects to /auth/login.
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use tower_sessions::Session;

use crate::auth::page::{LoginVariant, render};
use crate::auth::session::{clear_session, set_session_user_id};

/// `GET /auth/login` in dev mode — the one-click "Sign in as Admin" page.
pub async fn login_page() -> impl IntoResponse {
    Html(render(LoginVariant::Dev))
}

/// `POST /auth/dev-login` — look up the first admin user and log them in.
pub async fn dev_login_post(
    session: Session,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let state = crate::state::global_state().await;

    let row = sqlx::query!(
        r#"SELECT id, org_id, email, name,
                  org_role::text as "org_role!: String"
           FROM users WHERE org_role = $1 AND active = true LIMIT 1"#,
        horae_core::types::OrgRole::Admin as horae_core::types::OrgRole,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;

    let row = row.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No admin user found — run `horae seed` first.",
    ))?;

    // Rotate the session id on login to defeat session fixation (matches the
    // OIDC path).
    session.cycle_id().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to rotate session",
        )
    })?;
    set_session_user_id(&session, row.id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write session"))?;

    // Dispatch user_logged_in event (FR-019).
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserLoggedIn {
            occurred_at: chrono::Utc::now(),
            org_id: row.org_id,
            user: crate::plugin::event::UserPayload {
                id: row.id,
                email: row.email,
                name: row.name,
                org_role: row.org_role,
                method: Some("dev".into()),
            },
        });

    Ok(Redirect::to("/"))
}

/// `POST /auth/logout` — flush the session and redirect to login.
pub async fn logout_post(session: Session) -> impl IntoResponse {
    // Capture the user before clearing the session so the logout can be announced.
    if let Some(uid) = crate::auth::session::get_session_user_id(&session).await {
        let state = crate::state::global_state().await;
        if let Ok(user) = sqlx::query_as::<_, crate::models::User>(
            "SELECT id, org_id, email, name, oidc_subject, org_role,
                    cost_rate_cents, billable_rate_cents, active, created_at
             FROM users WHERE id = $1",
        )
        .bind(uid)
        .fetch_one(&state.db)
        .await
        {
            state
                .plugins
                .dispatch(crate::plugin::AppEvent::UserLoggedOut {
                    occurred_at: chrono::Utc::now(),
                    org_id: user.org_id,
                    user: crate::plugin::event::UserPayload {
                        id: user.id,
                        email: user.email,
                        name: user.name,
                        org_role: user.org_role.to_string(),
                        method: None,
                    },
                });
        }
    }
    clear_session(&session).await.ok();
    Redirect::to("/auth/login")
}
