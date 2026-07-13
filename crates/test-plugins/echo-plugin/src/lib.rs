use extism_pdk::*;

/// Handles the time_entry_created event — just returns the input as-is.
#[plugin_fn]
pub fn time_entry_created(input: String) -> FnResult<String> {
    Ok(input)
}

/// Handles the invoice_sent event.
#[plugin_fn]
pub fn invoice_sent(input: String) -> FnResult<String> {
    Ok(input)
}

/// Returns a dashboard widget.
#[plugin_fn]
pub fn dashboard_widget(_input: String) -> FnResult<String> {
    Ok(r#"{"widget":{"title":"Echo Plugin","body_format":"markdown","body":"Plugin is running."}}"#.into())
}
