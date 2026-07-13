use extism_pdk::*;

/// Always panics — tests that a failing plugin doesn't break the host.
#[plugin_fn]
pub fn time_entry_created(_input: String) -> FnResult<String> {
    panic!("deliberate plugin panic for testing failure isolation");
}
