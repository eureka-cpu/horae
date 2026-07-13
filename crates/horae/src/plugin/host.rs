use extism::{CurrentPlugin, Error, UserData, Val, ValType};

/// Register all Horae host functions on a plugin manifest.
/// These are the only capabilities a plugin receives (FR-020).
pub fn host_functions() -> Vec<extism::Function> {
    vec![extism::Function::new(
        "horae_log",
        [ValType::I64],
        [],
        UserData::new(()),
        horae_log,
    )]
}

/// `horae_log(level, message)` — structured logging annotated with the plugin name.
fn horae_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<()>,
) -> Result<(), Error> {
    let input: Vec<u8> = plugin.memory_get_val(&inputs[0])?;
    let msg = String::from_utf8_lossy(&input);

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
