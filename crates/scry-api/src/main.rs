mod auth_store;
mod error;
mod http_auth;
mod server;
mod service;
mod store;

use crate::auth_store::AuthStore;
use crate::error::AppError;
use crate::service::ProjectService;
use crate::store::{ArcStore, PostgresStore};
use clap::Parser;
use scry_core::config::{ScryConfig, ScrySecrets};
use std::path::PathBuf;
use std::sync::Arc;

/// The scry REST API server.
#[derive(Parser)]
#[command(
    name = "scry-api",
    about = "Serve the scry REST API over HTTP",
    version
)]
struct Cli {
    /// The address to bind, for example `127.0.0.1:9000`
    #[arg(long, value_name = "ADDR")]
    http: String,

    /// Read configuration from this file instead of the discovered default
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Read secrets from this file instead of the discovered default
    #[arg(long, value_name = "PATH")]
    secrets: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let config = ScryConfig::load(cli.config.as_deref())?;
    let secrets = ScrySecrets::load(cli.secrets.as_deref())?;
    let database_url = secrets.resolved_database_url(&config)?;
    let (store, auth_store) = connect_store(&database_url).await?;
    let service = ProjectService::new(store);

    server::serve_http(
        service,
        auth_store,
        &cli.http,
        secrets.admin_password.clone(),
    )
    .await
}

/// Open the Postgres store for the given database URL.
///
/// Only Postgres URLs (`postgres://` and `postgresql://`) are supported. The
/// returned auth store is a clone of the same connection pool, so refresh
/// tokens share the store's configuration.
async fn connect_store(database_url: &str) -> Result<(ArcStore, Arc<dyn AuthStore>), AppError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let postgres = PostgresStore::new(database_url).await?;
        let auth_store: Arc<dyn AuthStore> = Arc::new(postgres.clone());
        let store: ArcStore = Arc::new(postgres);
        Ok((store, auth_store))
    } else {
        Err(AppError::Config(format!(
            "unsupported database URL {database_url}: only postgres:// and postgresql:// are supported"
        )))
    }
}
