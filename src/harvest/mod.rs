// Harvest-compatible REST API surface, mounted at /harvest/v2.
//
// Tools like harvest-invoicer and harvest-exporter can be pointed at
//   https://horae.example.com/harvest
// and will call /harvest/v2/time_entries etc. as normal.
//
// Implemented in M8.5 — stubs only for now.

mod auth;

pub fn router() -> axum::Router {
    axum::Router::new()
        // Stub: will be populated in M8.5
        .nest("/harvest/v2", axum::Router::new())
}
