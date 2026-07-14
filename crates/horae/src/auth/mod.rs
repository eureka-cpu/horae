pub mod dev;
pub mod oidc;
pub mod session;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

/// Build the Axum sub-router for all `/auth/*` endpoints.
///
/// In dev mode (`DEV_LOGIN=1`) `/auth/login` serves the one-click admin page and
/// the `/auth/dev-login` bypass is registered. Otherwise `/auth/login` starts the
/// OIDC flow and `/auth/callback` completes it — the passwordless dev bypass is
/// **not** registered, so it can never be reached in production.
pub fn router(dev_login: bool) -> Router {
    let mut router = Router::new().route("/auth/logout", post(dev::logout_post));

    if dev_login {
        router = router
            .route("/auth/login", get(dev::login_page))
            .route("/auth/dev-login", post(dev::dev_login_post));
    } else {
        router = router
            .route("/auth/login", get(oidc::login))
            .route("/auth/callback", get(oidc::callback));
    }

    router
}

/// Create and migrate the Postgres-backed session store, then return a
/// `SessionManagerLayer` ready to be applied to the Axum router. `secure` marks
/// cookies `Secure` (HTTPS-only) — set it in production.
pub async fn make_session_layer(
    pool: PgPool,
    secure: bool,
) -> anyhow::Result<SessionManagerLayer<PostgresStore>> {
    let store = PostgresStore::new(pool);
    store.migrate().await?;

    let layer = SessionManagerLayer::new(store)
        .with_secure(secure)
        .with_expiry(Expiry::OnSessionEnd); // expires when the browser session ends

    Ok(layer)
}
