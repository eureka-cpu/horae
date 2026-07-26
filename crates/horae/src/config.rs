use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_level: String,
    /// When `DEV_LOGIN=1`, skip OIDC and log in as the seeded admin user.
    pub dev_login: bool,
    /// Secret for signing session cookies (set `SESSION_SECRET` in prod).
    pub session_secret: String,
    /// Directory containing plugin subdirectories (each with plugin.toml + *.wasm).
    pub plugins_dir: String,
    /// OIDC provider settings. `Some` only when all four env vars are set;
    /// production auth is enabled exactly when this is present and `dev_login`
    /// is false.
    pub oidc: Option<OidcConfig>,
    /// Mark session cookies `Secure` (send only over HTTPS). Set `SECURE_COOKIES=1`
    /// in production, where TLS is terminated in front of (or by) the app.
    pub secure_cookies: bool,
}

/// OIDC provider configuration, read from `HORAE_OIDC_ISSUER`,
/// `HORAE_OIDC_CLIENT_ID`, `HORAE_OIDC_CLIENT_SECRET`, and `HORAE_OIDC_REDIRECT_URL`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    /// Extra `aud` values to trust in the ID token beyond `client_id`. Some
    /// providers (e.g. Zitadel) add the project ID to `aud`; without listing it
    /// here the strict verifier rejects the token. Empty means strict
    /// (client_id only). Set `HORAE_OIDC_ADDITIONAL_AUDIENCES` (comma-separated).
    pub additional_audiences: Vec<String>,
    /// Label on the sign-in page's SSO button. Deployments front a named provider
    /// (e.g. "Continue with Okta", "Sign in with Google"), so this is overridable
    /// via `HORAE_OIDC_BUTTON_LABEL`; it defaults to [`DEFAULT_OIDC_BUTTON_LABEL`].
    pub button_label: String,
}

/// Default text on the OIDC sign-in button when `HORAE_OIDC_BUTTON_LABEL` is
/// unset — provider-agnostic, since Horae does not know the provider's name.
pub const DEFAULT_OIDC_BUTTON_LABEL: &str = "Continue with SSO";

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: std::env::var("HORAE_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("HORAE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/horae".into()),
            log_level: std::env::var("HORAE_LOG").unwrap_or_else(|_| "info".into()),
            dev_login: std::env::var("DEV_LOGIN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            session_secret: std::env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-me-in-production".into()),
            plugins_dir: std::env::var("HORAE_PLUGINS_DIR").unwrap_or_else(|_| "plugins".into()),
            oidc: OidcConfig::from_env(),
            secure_cookies: std::env::var("HORAE_SECURE_COOKIES")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        })
    }
}

impl OidcConfig {
    /// Returns `Some` only when all four OIDC env vars are present; a partial
    /// configuration is treated as "OIDC not configured" rather than a hard error,
    /// so `DEV_LOGIN` deployments need not set any of them.
    fn from_env() -> Option<Self> {
        Some(Self {
            issuer: non_empty("HORAE_OIDC_ISSUER")?,
            client_id: non_empty("HORAE_OIDC_CLIENT_ID")?,
            client_secret: non_empty("HORAE_OIDC_CLIENT_SECRET")?,
            redirect_url: non_empty("HORAE_OIDC_REDIRECT_URL")?,
            additional_audiences: non_empty("HORAE_OIDC_ADDITIONAL_AUDIENCES")
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            button_label: non_empty("HORAE_OIDC_BUTTON_LABEL")
                .unwrap_or_else(|| DEFAULT_OIDC_BUTTON_LABEL.into()),
        })
    }
}

/// An environment variable's value if it is set and non-empty.
fn non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
