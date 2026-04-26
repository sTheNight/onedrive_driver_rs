use std::net::SocketAddr;

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::AppState;

mod handler;
mod models;
mod service;
mod state;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_tracing();
    if let Err(e) = dotenvy::dotenv() {
        tracing::warn!("Failed to load .env file: {}", e);
    }

    let state = AppState::from_env().unwrap_or_else(|err| {
        tracing::error!("Failed to create AppState: {}", err);
        panic!("Could not create AppState")
    });

    let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any);
    let listen_port: u16 = 3000;
    let addr = SocketAddr::from(([127, 0, 0, 1], listen_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let app = Router::new()
        .route("/api/list", get(handler::get_file_list))
        .route("/api/list/", get(handler::get_file_list))
        .route("/api/list/{*path}", get(handler::get_file_list))
        .route("/api/download", get(handler::download_file))
        .layer(cors)
        .with_state(state);
    axum::serve(listener, app).await
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
