// Horae — Self-hostable time tracking
// Server entry: `horae serve` / `horae migrate`
// Web entry (WASM): `dioxus::launch(App)`

#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "server")]
mod cli;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod harvest;
#[cfg(feature = "server")]
mod reports;
#[cfg(feature = "server")]
mod seed;
#[cfg(feature = "server")]
mod state;

mod app;
mod components;
mod error;
mod models;
mod pages;
mod route;
mod server_fns;

#[cfg(feature = "server")]
fn main() -> anyhow::Result<()> {
    use clap::Parser;

    use cli::{Cli, Commands, MigrateAction};
    use config::AppConfig;

    let cli = Cli::parse();
    let cfg = AppConfig::from_env()?;

    match cli.command() {
        Commands::Migrate { action } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                match action {
                    MigrateAction::Run => {
                        tracing::info!("Running migrations...");
                        db::run_migrations(&pool).await?;
                        tracing::info!("Migrations complete.");
                    }
                    MigrateAction::Reset { confirm } => {
                        if !confirm {
                            eprintln!("Pass --confirm to reset the database.");
                            std::process::exit(1);
                        }
                        tracing::warn!("Resetting database...");
                        db::run_migrations(&pool).await?;
                    }
                }
                anyhow::Ok(())
            })?;
        }

        Commands::User { action } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                match action {
                    cli::UserAction::Create { email, name, role } => {
                        tracing::info!("Creating user {} ({})", name, email);
                        // TODO: insert user via OIDC subject or admin invite
                        let _ = (pool, email, name, role);
                    }
                    cli::UserAction::List => {
                        // TODO: query and print users
                        let _ = pool;
                        println!("No users found.");
                    }
                }
                anyhow::Ok(())
            })?;
        }

        Commands::Seed => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                seed::run(&pool).await?;
                seed::verify(&pool).await?;
                anyhow::Ok(())
            })?;
        }

        Commands::Serve(args) => {
            init_tracing(&cfg.log_level);

            // Resolve bind address: CLI args, overridden by IP/PORT env vars when
            // running under `dx serve` (which sets those for hot-reload proxying).
            let host = std::env::var("IP").unwrap_or_else(|_| args.host.clone());
            let port = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(args.port);
            let addr = format!("{}:{}", host, port);
            tracing::info!("Starting Horae on {addr}");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                use axum::routing::get;
                use dioxus::prelude::{DioxusRouterExt, ServeConfig};

                // Initialise DB + migrations eagerly so auth and server fns share the pool.
                let pool = db::create_pool(&cfg.database_url).await?;
                db::run_migrations(&pool).await?;
                state::init_state(pool.clone()).await;

                // Session middleware (Postgres-backed, idempotent migrate).
                let session_layer = auth::make_session_layer(pool.clone()).await?;

                // Build the Dioxus fullstack router (returns Router<()>), then
                // layer our own routes on top of it.
                let router = axum::Router::<dioxus::server::FullstackState>::new()
                    .serve_dioxus_application(ServeConfig::new(), app::App)
                    .route(
                        "/health",
                        get(|| async {
                            axum::Json(serde_json::json!({"status": "ok"}))
                        }),
                    )
                    .route("/api/reports/export/csv", get(reports::export_csv))
                    .route("/api/reports/export/xlsx", get(reports::export_xlsx))
                    .merge(auth::router())
                    .merge(harvest::router())
                    .layer(session_layer);

                let listener = tokio::net::TcpListener::bind(&addr).await?;
                tracing::info!("Listening on {addr}");
                axum::serve(listener, router).await?;
                anyhow::Ok(())
            })?;
        }
    }

    Ok(())
}

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(app::App);
}

#[cfg(not(any(feature = "server", feature = "web")))]
fn main() {
    eprintln!("Build with --features server or --features web (or use `dx serve`).");
    std::process::exit(1);
}

#[cfg(feature = "server")]
fn init_tracing(log_level: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .try_init();
}
