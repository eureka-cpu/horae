use std::collections::HashMap;
use std::time::Duration;

use extism::{CurrentPlugin, Error, UserData, Val, ValType};
use serde_json::Value;

/// Per-plugin state shared with the host functions. Only `horae_config_get`
/// reads it today; the others reach the DB pool through the global state.
#[derive(Clone)]
pub struct HostState {
    /// This plugin's own configuration (from the `[config]` table in its manifest).
    config: HashMap<String, String>,
}

/// The wall-clock bound on an outbound `horae_http_post`.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Register all Horae host functions for a plugin. These four capabilities are
/// the only ones a plugin receives (FR-020): structured logging, read-only data
/// lookups, outbound HTTP POST, and access to its own configuration — no
/// filesystem, no data writes, no ambient syscalls.
pub fn host_functions(config: HashMap<String, String>) -> Vec<extism::Function> {
    let state = UserData::new(HostState { config });
    vec![
        extism::Function::new("horae_log", [ValType::I64], [], state.clone(), horae_log),
        extism::Function::new(
            "horae_db_query",
            [ValType::I64],
            [ValType::I64],
            state.clone(),
            horae_db_query,
        ),
        extism::Function::new(
            "horae_http_post",
            [ValType::I64],
            [ValType::I64],
            state.clone(),
            horae_http_post,
        ),
        extism::Function::new(
            "horae_config_get",
            [ValType::I64],
            [ValType::I64],
            state,
            horae_config_get,
        ),
    ]
}

/// Read a plugin-memory argument as a UTF-8 string.
fn read_input(plugin: &mut CurrentPlugin, val: &Val) -> Result<String, Error> {
    let bytes: Vec<u8> = plugin.memory_get_val(val)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Hand a string back to the plugin as its return value (a memory offset).
fn write_output(plugin: &mut CurrentPlugin, out: &mut Val, s: &str) -> Result<(), Error> {
    let handle = plugin.memory_new(s)?;
    *out = plugin.memory_to_val(handle);
    Ok(())
}

/// `horae_log(level, message)` — structured logging annotated with the plugin name.
fn horae_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<HostState>,
) -> Result<(), Error> {
    let msg = read_input(plugin, &inputs[0])?;

    #[derive(serde::Deserialize)]
    struct LogMsg {
        level: Option<String>,
        message: String,
    }

    if let Ok(log) = serde_json::from_str::<LogMsg>(&msg) {
        let level = log.level.as_deref().unwrap_or("info");
        match level {
            "error" => tracing::error!(target: "plugin", "{}", log.message),
            "warn" => tracing::warn!(target: "plugin", "{}", log.message),
            "debug" => tracing::debug!(target: "plugin", "{}", log.message),
            _ => tracing::info!(target: "plugin", "{}", log.message),
        }
    } else {
        tracing::info!(target: "plugin", "{msg}");
    }

    Ok(())
}

/// `horae_db_query(request_json) -> rows_json` — read-only SQL lookup.
///
/// Input is `{"sql": "...", "params": [...]}`; output is a JSON array of row
/// objects, or `{"error": "..."}` on failure. Read-only is enforced two ways:
/// a cheap `SELECT`-prefix guard, and wrapping the query as a subquery, which
/// Postgres only accepts for a `SELECT` (FR-020: plugins MUST NOT modify data).
/// Failures are returned as JSON, never panicked, so a plugin call is isolated.
fn horae_db_query(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;
    let out = run_db_query(&input).unwrap_or_else(error_json);
    write_output(plugin, &mut outputs[0], &out)
}

fn run_db_query(input: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Request {
        sql: String,
        #[serde(default)]
        params: Vec<Value>,
    }

    let req: Request = serde_json::from_str(input).map_err(|e| format!("invalid request: {e}"))?;

    if !statement_is_read_only(&req.sql) {
        return Err("only a single SELECT statement is permitted".to_string());
    }

    let pool = crate::state::try_pool().ok_or("database is not available")?;
    let wrapped = wrap_select(&req.sql);

    // extism host functions are synchronous; bridge to the async pool on the
    // current multi-threaded runtime. Guard against a current-thread runtime
    // (e.g. a `#[tokio::test]` default), where `block_in_place` would panic.
    let handle =
        tokio::runtime::Handle::try_current().map_err(|_| "no async runtime".to_string())?;
    if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
        return Err("db query requires a multi-threaded runtime".to_string());
    }

    let rows: String = tokio::task::block_in_place(|| {
        handle.block_on(async {
            let mut q = sqlx::query_scalar::<sqlx::Postgres, String>(&wrapped);
            for p in &req.params {
                q = match p {
                    Value::Null => q.bind(Option::<String>::None),
                    Value::Bool(b) => q.bind(*b),
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            q.bind(i)
                        } else if let Some(f) = n.as_f64() {
                            q.bind(f)
                        } else {
                            q.bind(n.to_string())
                        }
                    }
                    Value::String(s) => q.bind(s.clone()),
                    other => q.bind(other.to_string()),
                };
            }
            q.fetch_one(&pool).await
        })
    })
    .map_err(|e| format!("query failed: {e}"))?;

    Ok(rows)
}

/// Whether `sql` is a single read-only `SELECT` statement. Leading whitespace and
/// `--` line comments are skipped; anything that is not `SELECT`/`WITH`, or that
/// packs a second statement after a `;`, is rejected.
fn statement_is_read_only(sql: &str) -> bool {
    let mut cleaned = String::with_capacity(sql.len());
    for line in sql.lines() {
        let line = match line.split_once("--") {
            Some((code, _comment)) => code,
            None => line,
        };
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    let cleaned = cleaned.trim();

    // Reject a trailing second statement: a `;` is allowed only at the very end.
    if let Some(idx) = cleaned.find(';')
        && idx != cleaned.len() - 1
    {
        return false;
    }

    let head = cleaned.trim_start().to_ascii_uppercase();
    head.starts_with("SELECT") || head.starts_with("WITH")
}

/// Wrap a validated SELECT so Postgres serialises the result to a single JSON
/// text value. The `(<sql>)` subquery form is only valid for a SELECT, which is
/// a second, database-enforced barrier against writes.
fn wrap_select(sql: &str) -> String {
    let trimmed = sql.trim().trim_end_matches(';');
    format!("SELECT coalesce(json_agg(_t), '[]'::json)::text FROM ({trimmed}) AS _t")
}

/// `horae_http_post(request_json) -> response_json` — outbound HTTP POST.
///
/// Input is `{"url": "...", "body": <json>}`; output is `{"status": u16,
/// "body": "..."}`, or `{"error": "..."}` when the request could not be made.
/// Bounded by [`HTTP_TIMEOUT`]. Any failure is returned as JSON, never panicked.
fn horae_http_post(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;
    let out = run_http_post(&input).unwrap_or_else(error_json);
    write_output(plugin, &mut outputs[0], &out)
}

fn run_http_post(input: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Request {
        url: String,
        #[serde(default)]
        body: Value,
    }

    let req: Request = serde_json::from_str(input).map_err(|e| format!("invalid request: {e}"))?;
    let body = req.body.to_string();

    let agent = ureq::AgentBuilder::new().timeout(HTTP_TIMEOUT).build();

    // ureq returns `Err(Status(..))` for non-2xx; both carry a usable response.
    let response = match agent
        .post(&req.url)
        .set("Content-Type", "application/json")
        .send_string(&body)
    {
        Ok(r) => r,
        Err(ureq::Error::Status(_code, r)) => r,
        Err(ureq::Error::Transport(t)) => return Err(format!("request failed: {t}")),
    };

    Ok(http_response_json(
        response.status(),
        response.into_string().unwrap_or_default(),
    ))
}

/// Shape an HTTP status + body into the `{"status", "body"}` response contract.
fn http_response_json(status: u16, body: String) -> String {
    serde_json::json!({ "status": status, "body": body }).to_string()
}

/// `horae_config_get(request_json) -> value_json` — read this plugin's own config.
///
/// Input is `{"key": "..."}`; output is the JSON string value, or JSON `null`
/// when the key is unset. A plugin can only read its own `[config]` table.
fn horae_config_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;

    #[derive(serde::Deserialize)]
    struct Request {
        key: String,
    }

    let value = match serde_json::from_str::<Request>(&input) {
        Ok(req) => {
            let store = user_data.get()?;
            let state = store.lock().unwrap_or_else(|p| p.into_inner());
            state.config.get(&req.key).cloned()
        }
        Err(_) => None,
    };

    let out = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
    write_output(plugin, &mut outputs[0], &out)
}

/// A `{"error": "..."}` JSON string for a failed host-function call.
fn error_json(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod statement_is_read_only {
        use super::*;

        #[test]
        fn plain_select_is_allowed() {
            assert!(statement_is_read_only("SELECT 1"));
            assert!(statement_is_read_only("  select id from projects  "));
        }

        #[test]
        fn a_cte_select_is_allowed() {
            assert!(statement_is_read_only(
                "WITH t AS (SELECT 1 AS n) SELECT n FROM t"
            ));
        }

        #[test]
        fn writes_are_rejected() {
            assert!(!statement_is_read_only("INSERT INTO users VALUES (1)"));
            assert!(!statement_is_read_only("UPDATE users SET name = 'x'"));
            assert!(!statement_is_read_only("DELETE FROM users"));
            assert!(!statement_is_read_only("DROP TABLE users"));
        }

        #[test]
        fn a_second_statement_is_rejected() {
            assert!(!statement_is_read_only("SELECT 1; DROP TABLE users"));
        }

        #[test]
        fn a_single_trailing_semicolon_is_allowed() {
            assert!(statement_is_read_only("SELECT 1;"));
        }

        #[test]
        fn a_write_hidden_behind_a_comment_is_rejected() {
            // The comment is stripped, exposing the real leading keyword.
            assert!(!statement_is_read_only("-- harmless\nDELETE FROM users"));
        }
    }

    mod wrap_select {
        use super::*;

        #[test]
        fn wraps_as_a_json_aggregating_subquery() {
            assert_eq!(
                wrap_select("SELECT id FROM projects"),
                "SELECT coalesce(json_agg(_t), '[]'::json)::text FROM (SELECT id FROM projects) AS _t"
            );
        }

        #[test]
        fn strips_a_trailing_semicolon_before_wrapping() {
            assert_eq!(
                wrap_select("SELECT 1;"),
                "SELECT coalesce(json_agg(_t), '[]'::json)::text FROM (SELECT 1) AS _t"
            );
        }
    }

    mod config_lookup {
        use super::*;

        fn state() -> HostState {
            HostState {
                config: HashMap::from([("webhook_url".to_string(), "https://x".to_string())]),
            }
        }

        #[test]
        fn a_set_key_is_returned() {
            assert_eq!(
                state().config.get("webhook_url").cloned(),
                Some("https://x".to_string())
            );
        }

        #[test]
        fn an_unset_key_is_none() {
            assert_eq!(state().config.get("missing").cloned(), None);
        }
    }

    mod response_shaping {
        use super::*;

        #[test]
        fn http_response_carries_status_and_body() {
            let json = http_response_json(200, "ok".to_string());
            assert!(json.contains("\"status\":200"));
            assert!(json.contains("\"body\":\"ok\""));
        }

        #[test]
        fn error_json_wraps_the_message() {
            assert_eq!(error_json("boom".to_string()), "{\"error\":\"boom\"}");
        }
    }
}
