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
mod plugin;
#[cfg(feature = "server")]
mod render;
#[cfg(feature = "server")]
mod reports;
#[cfg(feature = "server")]
mod scheduler;
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
                        let org_role: horae_core::types::OrgRole =
                            role.parse().map_err(|e: String| anyhow::anyhow!(e))?;

                        let org_id = sqlx::query_scalar!("SELECT id FROM organizations LIMIT 1")
                            .fetch_optional(&pool)
                            .await?
                            .ok_or_else(|| {
                                anyhow::anyhow!("No organization found. Run `seed` first.")
                            })?;

                        let id = uuid::Uuid::now_v7();
                        sqlx::query!(
                            "INSERT INTO users (id, org_id, email, name, org_role) \
                             VALUES ($1, $2, $3, $4, $5)",
                            id,
                            org_id,
                            email,
                            name,
                            org_role as horae_core::types::OrgRole,
                        )
                        .execute(&pool)
                        .await?;

                        println!("Created user: {} ({}) [{}]", name, email, role);
                    }
                    cli::UserAction::List => {
                        let users = sqlx::query!(
                            "SELECT name, email, org_role::text as role, active \
                             FROM users ORDER BY name ASC"
                        )
                        .fetch_all(&pool)
                        .await?;

                        if users.is_empty() {
                            println!("No users found.");
                        } else {
                            println!("{:<30} {:<30} {:<10} STATUS", "NAME", "EMAIL", "ROLE");
                            for u in &users {
                                let status = if u.active { "active" } else { "inactive" };
                                println!(
                                    "{:<30} {:<30} {:<10} {}",
                                    u.name,
                                    u.email,
                                    u.role.as_deref().unwrap_or("?"),
                                    status,
                                );
                            }
                        }
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

                // Load plugins from the configured directory (FR-018).
                let plugins_dir = std::path::Path::new(&cfg.plugins_dir);
                let registry = std::sync::Arc::new(plugin::PluginRegistry::load(plugins_dir));
                state::init_state(pool.clone(), registry, cfg.oidc.clone()).await;

                // Start the background poller for forgotten timers (US3).
                scheduler::spawn(state::global_state().await);

                // Session middleware (Postgres-backed, idempotent migrate).
                let session_layer =
                    auth::make_session_layer(pool.clone(), cfg.secure_cookies).await?;

                // Build the Dioxus fullstack router (returns Router<()>), then
                // layer our own routes on top of it.
                let router = axum::Router::<dioxus::server::FullstackState>::new()
                    .serve_dioxus_application(ServeConfig::new(), app::App)
                    .route(
                        "/health",
                        get(|| async { axum::Json(serde_json::json!({"status": "ok"})) }),
                    )
                    .route("/api/reports/export/csv", get(reports::export_csv))
                    .route("/api/reports/export/xlsx", get(reports::export_xlsx))
                    .route(
                        "/api/projects/export/csv",
                        get(reports::export_projects_csv),
                    )
                    .route(
                        "/api/projects/export/xlsx",
                        get(reports::export_projects_xlsx),
                    )
                    .route(
                        "/api/invoices/{id}/export/csv",
                        get(reports::export_invoice_csv),
                    )
                    .route(
                        "/api/invoices/{id}/export/xlsx",
                        get(reports::export_invoice_xlsx),
                    )
                    .route(
                        "/api/invoices/{id}/export/pdf",
                        get(reports::export_invoice_pdf),
                    )
                    .merge(auth::router(cfg.dev_login))
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
