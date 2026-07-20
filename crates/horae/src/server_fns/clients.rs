//! Client server functions.

use super::*;

// ── Clients ──────────────────────────────────────────────────────────────────

/// Lists clients. With `include_inactive = false` only active clients are
/// returned (the set shown in new-entry pickers); pass `true` for the management
/// view that also needs to reactivate deactivated clients.
#[server]
pub async fn list_clients(include_inactive: bool) -> Result<Vec<Client>, ServerFnError> {
    let state = crate::state::global_state().await;

    let clients = sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients
         WHERE ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(clients)
}

#[server]
pub async fn create_client(
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let client = sqlx::query_as!(
        Client,
        r#"INSERT INTO clients (id, org_id, name, currency, address, tax_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        name,
        currency,
        address.as_deref(),
        tax_id.as_deref(),
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::ClientCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            client: client_payload(&client),
        });
    Ok(client)
}

#[server]
pub async fn update_client(
    client_id: String,
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    // Detect a real change so a no-op update emits nothing (FR-012).
    let changed: Option<bool> = sqlx::query_scalar::<_, bool>(
        "SELECT (name IS DISTINCT FROM $3 OR currency IS DISTINCT FROM $4
                 OR address IS DISTINCT FROM $5 OR tax_id IS DISTINCT FROM $6)
         FROM clients WHERE id = $1 AND org_id = $2",
    )
    .bind(client_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(&currency)
    .bind(&address)
    .bind(&tax_id)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    let client = sqlx::query_as::<_, Client>(
        "UPDATE clients SET name = $3, currency = $4, address = $5, tax_id = $6
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, currency, address, tax_id, active, created_at",
    )
    .bind(client_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(&currency)
    .bind(&address)
    .bind(&tax_id)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;

    if changed == Some(true) {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::ClientUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                client: client_payload(&client),
            });
    }
    Ok(client)
}

/// Activate or deactivate a client. Deactivated clients are hidden from
/// new-entry pickers but remain linked to existing projects and entries (FR-011).
#[server]
pub async fn set_client_active(client_id: String, active: bool) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    // Detect a real flip so a no-op set emits nothing (FR-012).
    let was_active: Option<bool> =
        sqlx::query_scalar::<_, bool>("SELECT active FROM clients WHERE id = $1 AND org_id = $2")
            .bind(client_id)
            .bind(manager.org_id)
            .fetch_optional(&state.db)
            .await
            .map_err(server_err)?;

    let client = sqlx::query_as::<_, Client>(
        "UPDATE clients SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, currency, address, tax_id, active, created_at",
    )
    .bind(client_id)
    .bind(manager.org_id)
    .bind(active)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;

    if let Some(t) = crate::plugin::event::active_transition(was_active, active) {
        let occurred_at = chrono::Utc::now();
        let client = client_payload(&client);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::ClientReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    client,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::ClientDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    client,
                }
            }
        });
    }
    Ok(client)
}
