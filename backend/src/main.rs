mod api;
mod models;
mod db;
mod services;

#[cfg(test)]
mod tests;

use anyhow::Result;
use axum::Router;
use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{info, instrument};
use tracing_subscriber;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rdumper-backend")]
#[command(about = "rDumper - Rust GUI Wrapper for mydumper/myloader")]
struct Cli {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long, default_value = "3000")]
    port: u16,

    #[arg(long, default_value = "sqlite://data/db/rdumper.db")]
    database_url: String,

    #[arg(long, default_value = "./data/backups")]
    backup_dir: String,

    #[arg(long, default_value = "./data/logs")]
    log_dir: String,

    #[arg(long, default_value = "../frontend/dist")]
    static_dir: String,
}

fn ensure_sqlite_file(url: &str) -> std::io::Result<()> {
    // "sqlite://data/db/rdumper.db" → "data/db/rdumper.db"
    let path = url.strip_prefix("sqlite://").unwrap_or(url);

    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    if !p.exists() {
        fs::File::create(p)?; // leere Datei anlegen
    }
    Ok(())
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    info!("Starting rDumper backend server");
    info!("Database URL: {}", cli.database_url);
    info!("Backup directory: {}", cli.backup_dir);
    info!("Log directory: {}", cli.log_dir);

    // Create backup and log directories if they don't exist
    std::fs::create_dir_all(&cli.backup_dir)?;
    std::fs::create_dir_all(&cli.log_dir)?;

    // Set environment variables for services
    std::env::set_var("BACKUP_DIR", &cli.backup_dir);
    std::env::set_var("LOG_DIR", &cli.log_dir);

    // Initialize database
    ensure_sqlite_file(&cli.database_url)?;
    let pool = db::create_database_pool(&cli.database_url).await?;
    info!("Database connection established");

    // Create API routes
    let api_routes = api::create_routes(pool.clone());

    // Create main application
    let app = Router::new()
        .merge(api_routes)
        .nest_service("/", ServeDir::new(&cli.static_dir))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", cli.host, cli.port)).await?;
    info!("Server listening on {}:{}", cli.host, cli.port);

    axum::serve(listener, app).await?;

    Ok(())
}