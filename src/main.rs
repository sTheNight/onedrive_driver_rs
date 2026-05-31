use crate::state::AppState;
use anyhow::{Context, Result};
use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{get, post, put},
};
use clap::{Parser, Subcommand};
use std::{io, net::SocketAddr, path::Path};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod database;
mod entity;
mod error;
mod handler;
mod models;
mod service;
mod state;
mod utils;

#[derive(Parser, Debug)]
#[command(name = "onedrive_driver_rs")]
#[command(version)]
#[command(about = "OneDrive Driver server and management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start,
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start) => {
            start_server().await?;
        }
        Some(Commands::Reset) => {
            reset_config().await?;
        }
        None => {}
    }
    Ok(())
}

async fn reset_config() -> Result<()> {
    let path = Path::new("./onedrive_driver.sqlite");

    match std::fs::remove_file(path) {
        Ok(()) => {
            println!("Removed file: {}", path.display());
            Ok(())
        }

        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            println!("File does not exist, skipped: {}", path.display());
            Ok(())
        }

        Err(err) => Err(err).with_context(|| format!("failed to remove file: {}", path.display())),
    }
}

async fn start_server() -> Result<()> {
    init_tracing();
    if let Err(e) = dotenvy::dotenv() {
        tracing::warn!("Failed to load .env file: {}", e);
    }

    let _db = database::init_database().await?;

    let state = AppState::init(_db).unwrap_or_else(|err| {
        tracing::error!("Failed to create AppState: {}", err);
        panic!("Could not create AppState")
    });

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let listen_port = utils::get_env("LISTEN_PORT", 3000);
    let addr = SocketAddr::from(([127, 0, 0, 1], listen_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let spa_fallback_service =
        ServeDir::new("dist").not_found_service(ServeFile::new("dist/index.html"));

    let app = Router::new()
        .route("/api/list", get(handler::file_list::get_file_list))
        .route("/api/list/", get(handler::file_list::get_file_list))
        .route("/api/list/{*path}", get(handler::file_list::get_file_list))
        .route(
            "/api/download/{*path}",
            get(handler::file_list::download_file),
        )
        .route("/api/admin/init", post(handler::admin::init_admin_user))
        .route("/api/admin/login", post(handler::admin::login))
        .route(
            "/api/admin/onedrive-config",
            put(handler::admin::update_onedrive_config),
        )
        .route(
            "/api/admin/onedrive-config",
            get(handler::admin::get_onedrive_config),
        )
        .route("/api/admin/me", get(handler::admin::get_current_user))
        .layer(cors)
        .fallback_service(spa_fallback_service)
        .with_state(state);
    tracing::info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
