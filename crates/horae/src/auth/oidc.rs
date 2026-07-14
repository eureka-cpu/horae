//! Production OIDC authentication (authorization-code flow with PKCE).
//!
//! `GET /auth/login` redirects to the provider; `GET /auth/callback` completes
//! the exchange, verifies the ID token, and maps the verified identity onto an
//! **existing** Horae user. Accounts are created by admins (FR-002), so a first
//! OIDC login *links* the subject to a user matched by verified email — it never
//! auto-provisions. Deactivated users are denied at sign-in.

use axum::extract::Query;
use axum::response::{IntoResponse, Redirect};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::session::set_session_user_id;
use crate::config::OidcConfig;

/// Session key holding the in-flight authorization request's CSRF, PKCE, and
/// nonce secrets between `/auth/login` and `/auth/callback`.
const OIDC_FLOW_KEY: &str = "oidc_flow";

/// The one-time secrets that tie a callback back to the login that started it.
#[derive(Serialize, Deserialize)]
struct OidcFlow {
    csrf: String,
    pkce_verifier: String,
    nonce: String,
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// `GET /auth/login` for OIDC deployments: start the authorization-code flow.
/// Stashes the CSRF/PKCE/nonce secrets in the session and redirects to the
/// provider. Falls back to `/auth/login` on any setup error.
pub async fn login(session: Session) -> axum::response::Response {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        // Neither dev-login nor OIDC configured. Return a plain error rather than
        // redirecting back to `/auth/login`, which would loop.
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication is not configured.",
        )
            .into_response();
    };

    let build = tokio::task::spawn_blocking(move || build_authorization(&cfg)).await;
    let auth = match build {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            tracing::error!("OIDC login setup failed: {e}");
            return Redirect::to("/auth/login").into_response();
        }
        Err(e) => {
            tracing::error!("OIDC login task panicked: {e}");
            return Redirect::to("/auth/login").into_response();
        }
    };

    let flow = OidcFlow {
        csrf: auth.csrf,
        pkce_verifier: auth.pkce_verifier,
        nonce: auth.nonce,
    };
    if let Err(e) = session.insert(OIDC_FLOW_KEY, flow).await {
        tracing::error!("failed to store OIDC flow in session: {e}");
        return Redirect::to("/auth/login").into_response();
    }

    Redirect::to(&auth.url).into_response()
}

/// `GET /auth/callback`: validate state, exchange the code (PKCE), verify the
/// ID token and nonce, then resolve the identity onto a Horae user.
pub async fn callback(session: Session, Query(params): Query<CallbackParams>) -> impl IntoResponse {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        return Redirect::to("/auth/login");
    };

    // The flow secrets are single-use: consume them regardless of outcome.
    let flow: Option<OidcFlow> = session.get(OIDC_FLOW_KEY).await.ok().flatten();
    let _ = session.remove::<OidcFlow>(OIDC_FLOW_KEY).await;
    let Some(flow) = flow else {
        tracing::warn!("OIDC callback with no in-flight flow");
        return Redirect::to("/auth/login");
    };

    if params.error.is_some() {
        tracing::warn!("OIDC provider returned error: {:?}", params.error);
        return Redirect::to("/auth/login");
    }

    // CSRF: the returned `state` must equal the value we generated at login.
    let (Some(returned_state), Some(code)) = (params.state, params.code) else {
        tracing::warn!("OIDC callback missing state or code");
        return Redirect::to("/auth/login");
    };
    if returned_state != flow.csrf {
        tracing::warn!("OIDC callback state mismatch (possible CSRF)");
        return Redirect::to("/auth/login");
    }

    // Exchange the code and verify the ID token off the async runtime (blocking).
    let identity = tokio::task::spawn_blocking(move || {
        exchange_and_verify(&cfg, code, &flow.pkce_verifier, &flow.nonce)
    })
    .await;
    let identity = match identity {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            tracing::warn!("OIDC token exchange/verification failed: {e}");
            return Redirect::to("/auth/login");
        }
        Err(e) => {
            tracing::error!("OIDC callback task panicked: {e}");
            return Redirect::to("/auth/login");
        }
    };

    match resolve_and_login(state, &session, &identity).await {
        Ok(true) => Redirect::to("/"),
        Ok(false) => Redirect::to("/auth/login"),
        Err(e) => {
            tracing::error!("OIDC user resolution failed: {e}");
            Redirect::to("/auth/login")
        }
    }
}

/// The verified identity claims we act on.
struct Identity {
    subject: String,
    email: Option<String>,
    email_verified: bool,
}

/// A candidate user row for the resolution decision.
struct Candidate {
    id: Uuid,
    active: bool,
}

/// What a callback should do with the verified identity.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    /// Log in this user; `link_subject` = write the OIDC subject onto the row
    /// (true on a first, email-matched login).
    Login { id: Uuid, link_subject: bool },
    /// No Horae account matches — accounts are admin-created, so deny (FR-002).
    DenyNoUser,
    /// The matched account is deactivated — deny sign-in (FR-002).
    DenyInactive,
}

/// Pure resolution: match by subject first, then (only for a verified email) by
/// email. Deactivated matches are denied; an unmatched identity is denied rather
/// than provisioned.
fn resolve_identity(
    by_subject: Option<Candidate>,
    email_verified: bool,
    by_email: Option<Candidate>,
) -> Resolution {
    if let Some(u) = by_subject {
        return if u.active {
            Resolution::Login {
                id: u.id,
                link_subject: false,
            }
        } else {
            Resolution::DenyInactive
        };
    }
    if email_verified && let Some(u) = by_email {
        return if u.active {
            Resolution::Login {
                id: u.id,
                link_subject: true,
            }
        } else {
            Resolution::DenyInactive
        };
    }
    Resolution::DenyNoUser
}

/// Look up the identity, apply [`resolve_identity`], and on success write the
/// session and dispatch `user_logged_in`. Returns `Ok(true)` when logged in.
async fn resolve_and_login(
    state: &'static crate::state::AppState,
    session: &Session,
    identity: &Identity,
) -> anyhow::Result<bool> {
    let by_subject = fetch_user(state, "oidc_subject", &identity.subject).await?;
    let by_email = match (identity.email_verified, &identity.email) {
        (true, Some(email)) => fetch_user(state, "email", email).await?,
        _ => None,
    };

    let resolution = resolve_identity(
        by_subject.as_ref().map(candidate),
        identity.email_verified,
        by_email.as_ref().map(candidate),
    );

    let (user, link_subject) = match resolution {
        Resolution::Login { id, link_subject } => {
            // The matched user is whichever lookup produced `id`.
            let user = by_subject
                .into_iter()
                .chain(by_email)
                .find(|u| u.id == id)
                .ok_or_else(|| anyhow::anyhow!("resolved user vanished"))?;
            (user, link_subject)
        }
        Resolution::DenyNoUser => {
            tracing::warn!("OIDC login denied: no account for subject/email");
            return Ok(false);
        }
        Resolution::DenyInactive => {
            tracing::warn!("OIDC login denied: account is deactivated");
            return Ok(false);
        }
    };

    if link_subject {
        sqlx::query("UPDATE users SET oidc_subject = $1 WHERE id = $2")
            .bind(&identity.subject)
            .bind(user.id)
            .execute(&state.db)
            .await?;
    }

    set_session_user_id(session, user.id).await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserLoggedIn {
            occurred_at: chrono::Utc::now(),
            org_id: user.org_id,
            user: crate::plugin::event::UserPayload {
                id: user.id,
                email: user.email.clone(),
                name: user.name.clone(),
                org_role: user.org_role.to_string(),
                method: Some("oidc".into()),
            },
        });

    Ok(true)
}

fn candidate(u: &crate::models::User) -> Candidate {
    Candidate {
        id: u.id,
        active: u.active,
    }
}

/// Fetch a user by a single indexed column (`oidc_subject` or `email`). The
/// column name is a fixed internal literal, never user input.
async fn fetch_user(
    state: &crate::state::AppState,
    column: &str,
    value: &str,
) -> anyhow::Result<Option<crate::models::User>> {
    let sql = format!(
        "SELECT id, org_id, email, name, oidc_subject, org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users WHERE {column} = $1"
    );
    let user = sqlx::query_as::<_, crate::models::User>(&sql)
        .bind(value)
        .fetch_optional(&state.db)
        .await?;
    Ok(user)
}

/// The authorization request to send the user to.
struct Authorization {
    url: String,
    csrf: String,
    pkce_verifier: String,
    nonce: String,
}

/// Discover the provider and build the authorization URL + one-time secrets.
/// Blocking (network discovery) — call from `spawn_blocking`.
fn build_authorization(cfg: &OidcConfig) -> anyhow::Result<Authorization> {
    let client = discover_client(cfg)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok(Authorization {
        url: url.to_string(),
        csrf: csrf.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        nonce: nonce.secret().clone(),
    })
}

/// Exchange the authorization code and verify the ID token against `nonce`.
/// Blocking (network token request) — call from `spawn_blocking`.
fn exchange_and_verify(
    cfg: &OidcConfig,
    code: String,
    pkce_verifier: &str,
    nonce: &str,
) -> anyhow::Result<Identity> {
    let client = discover_client(cfg)?;

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
        .request(openidconnect::ureq::http_client)?;

    let id_token = token
        .id_token()
        .ok_or_else(|| anyhow::anyhow!("provider returned no ID token"))?;

    let claims = id_token.claims(&client.id_token_verifier(), &Nonce::new(nonce.to_string()))?;

    Ok(Identity {
        subject: claims.subject().as_str().to_string(),
        email: claims.email().map(|e| e.as_str().to_string()),
        email_verified: claims.email_verified().unwrap_or(false),
    })
}

/// Discover provider metadata and build a client. Blocking (network).
fn discover_client(cfg: &OidcConfig) -> anyhow::Result<CoreClient> {
    let issuer = IssuerUrl::new(cfg.issuer.clone())?;
    let metadata = CoreProviderMetadata::discover(&issuer, openidconnect::ureq::http_client)?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(cfg.redirect_url.clone())?);
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(id: Uuid) -> Candidate {
        Candidate { id, active: true }
    }
    fn inactive(id: Uuid) -> Candidate {
        Candidate { id, active: false }
    }

    #[test]
    fn subject_match_logs_in_without_relinking() {
        let id = Uuid::now_v7();
        assert_eq!(
            resolve_identity(Some(active(id)), true, None),
            Resolution::Login {
                id,
                link_subject: false
            }
        );
    }

    #[test]
    fn first_login_links_subject_via_verified_email() {
        let id = Uuid::now_v7();
        assert_eq!(
            resolve_identity(None, true, Some(active(id))),
            Resolution::Login {
                id,
                link_subject: true
            }
        );
    }

    #[test]
    fn unverified_email_is_not_matched() {
        let id = Uuid::now_v7();
        // Even with an email-matched candidate, an unverified email must not link.
        assert_eq!(
            resolve_identity(None, false, Some(active(id))),
            Resolution::DenyNoUser
        );
    }

    #[test]
    fn no_account_is_denied_not_provisioned() {
        assert_eq!(resolve_identity(None, true, None), Resolution::DenyNoUser);
    }

    #[test]
    fn deactivated_subject_match_is_denied() {
        let id = Uuid::now_v7();
        assert_eq!(
            resolve_identity(Some(inactive(id)), true, None),
            Resolution::DenyInactive
        );
    }

    #[test]
    fn deactivated_email_match_is_denied() {
        let id = Uuid::now_v7();
        assert_eq!(
            resolve_identity(None, true, Some(inactive(id))),
            Resolution::DenyInactive
        );
    }

    #[test]
    fn subject_match_takes_precedence_over_email() {
        let subject_id = Uuid::now_v7();
        let email_id = Uuid::now_v7();
        assert_eq!(
            resolve_identity(Some(active(subject_id)), true, Some(active(email_id))),
            Resolution::Login {
                id: subject_id,
                link_subject: false
            }
        );
    }
}
