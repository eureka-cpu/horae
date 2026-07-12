/// DEV_LOGIN handlers — only reachable when DEV_LOGIN=1.
///
/// GET  /auth/login      → HTML page with a one-click "Sign in as Admin" button.
/// POST /auth/dev-login  → sets session to the seeded admin user, redirects to /.
/// POST /auth/logout     → flushes the session, redirects to /auth/login.
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use tower_sessions::Session;

use crate::auth::session::{clear_session, set_session_user_id};

static DEV_LOGIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Sign In — Horae</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,600;1,6..72,400&family=Instrument+Sans:wght@400;500;600&display=swap" rel="stylesheet">
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: #100F0C; color: #EFEAE0;
      font-family: 'Instrument Sans', system-ui, sans-serif;
      display: flex; align-items: center; justify-content: center;
      min-height: 100vh;
    }
    .card {
      background: #1A1813; border: 1px solid #322E26; border-radius: 20px;
      padding: 2.5rem 2rem; width: 360px; text-align: center;
    }
    .logo {
      font-family: 'Newsreader', Georgia, serif; font-size: 2rem; font-weight: 600;
      color: #4FB79A; margin-bottom: 0.4rem; letter-spacing: -0.02em;
    }
    .subtitle {
      color: #A29C8D; font-size: 0.875rem; margin-bottom: 2rem;
    }
    .btn {
      display: block; width: 100%; padding: 0.75rem 1.5rem;
      border-radius: 8px; font-size: 0.875rem; font-weight: 600;
      cursor: pointer; border: none; font-family: inherit; transition: background 0.15s;
    }
    .btn-primary { background: #4FB79A; color: #0d211b; }
    .btn-primary:hover { background: #3D9E84; }
    .dev-badge {
      display: inline-block; margin-top: 1.25rem; padding: 0.25rem 0.625rem;
      border-radius: 9999px; font-size: 0.7rem; font-weight: 600; letter-spacing: 0.05em;
      background: rgba(79,183,154,0.14); color: #4FB79A; border: 1px solid rgba(79,183,154,0.3);
      text-transform: uppercase;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="logo">Horae</div>
    <p class="subtitle">Sign in to your workspace</p>
    <form method="POST" action="/auth/dev-login">
      <button type="submit" class="btn btn-primary">Sign in as Admin</button>
    </form>
    <span class="dev-badge">Dev mode</span>
  </div>
</body>
</html>"#;

/// `GET /auth/login` — serve the login page.
///
/// When `DEV_LOGIN=1` this returns the one-click dev login page.
/// When OIDC is configured (future) it redirects to the provider.
pub async fn login_page() -> impl IntoResponse {
    Html(DEV_LOGIN_HTML)
}

/// `POST /auth/dev-login` — look up the first admin user and log them in.
pub async fn dev_login_post(
    session: Session,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let state = crate::state::global_state().await;

    let row = sqlx::query!(
        "SELECT id FROM users WHERE org_role = $1 AND active = true LIMIT 1",
        horae_core::types::OrgRole::Admin as horae_core::types::OrgRole,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;

    let row = row.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No admin user found — run `horae seed` first.",
    ))?;
    let id = row.id;

    set_session_user_id(&session, id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write session"))?;

    Ok(Redirect::to("/"))
}

/// `POST /auth/logout` — flush the session and redirect to login.
pub async fn logout_post(session: Session) -> impl IntoResponse {
    clear_session(&session).await.ok();
    Redirect::to("/auth/login")
}
