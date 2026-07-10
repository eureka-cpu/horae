// OIDC authentication — stub for M3.5 / future milestone.
//
// Full implementation will use the `openidconnect` crate to:
// - Discover provider metadata from `OIDC_ISSUER`
// - Build a client from `OIDC_CLIENT_ID` / `OIDC_CLIENT_SECRET`
// - Handle the authorization code flow via `/auth/callback`
// - Upsert users by `oidc_subject` on successful token exchange
