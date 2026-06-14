use axum::{Router, routing::get};
use chat_server::{AppState};
use sea_orm::Database;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

use crate::api::init_api_router;

mod api;
mod db;
mod entity;
mod middleware;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db = Database::connect("sqlite://users.sqlite?mode=rwc")
        .await
        .unwrap();
    db::init_user_table(&db).await;

    let (tx, _rx) = broadcast::channel::<String>(20);

    let app_state = AppState { db, tx };

    let api_router = init_api_router(app_state);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api_router)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}
