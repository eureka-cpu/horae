// Horae — Self-hostable time tracking
// Server entry: `horae serve` / `horae migrate`
// Web entry (WASM): `dioxus::launch(App)`

#[cfg(feature = "server")]
mod cli;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod plugin;
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
                        // TODO: insert user with argon2 password hash
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

        Commands::Serve(args) => {
            init_tracing(&cfg.log_level);
            tracing::info!("Starting Horae on {}:{}", args.host, args.port);

            // dioxus::LaunchBuilder reads the bind address from the IP / PORT env vars
            // (via dioxus_cli_config::fullstack_address_or_localhost).
            // When launched by `dx serve`, those vars are already set — don't overwrite them.
            // When launched manually, fall back to the CLI args.
            #[allow(unsafe_code)]
            unsafe {
                if std::env::var("IP").is_err() {
                    std::env::set_var("IP", &args.host);
                }
                if std::env::var("PORT").is_err() {
                    std::env::set_var("PORT", args.port.to_string());
                }
            }

            // The AppState (DB pool + plugins) is initialised lazily on the first
            // server-function call via state::global_state(), inside dioxus's runtime.
            dioxus::LaunchBuilder::new().launch(app::App);
        }
    }

    Ok(())
}

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(app::App);
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
