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
pub fn router() -> Router {
    Router::new()
        .route("/auth/login", get(dev::login_page))
        .route("/auth/dev-login", post(dev::dev_login_post))
        .route("/auth/logout", post(dev::logout_post))
}

/// Create and migrate the Postgres-backed session store, then return a
/// `SessionManagerLayer` ready to be applied to the Axum router.
pub async fn make_session_layer(
    pool: PgPool,
) -> anyhow::Result<SessionManagerLayer<PostgresStore>> {
    let store = PostgresStore::new(pool);
    store.migrate().await?;

    let layer = SessionManagerLayer::new(store)
        .with_secure(false) // set true when TLS is terminated at the app (prod)
        .with_expiry(Expiry::OnSessionEnd); // expires when the browser session ends

    Ok(layer)
}
