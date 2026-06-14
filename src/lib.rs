pub mod api;
pub mod db;
pub mod entity;
pub mod middleware;

use axum::{Router, extract::FromRef, routing::get};
use sea_orm::Database;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub tx: broadcast::Sender<String>,
}

/// 构建测试用 App（使用内存数据库）
pub async fn build_test_app() -> Router {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    db::init_user_table(&db).await;

    let (tx, _rx) = broadcast::channel::<String>(20);

    let app_state = AppState { db, tx };

    let api_router = api::init_api_router(app_state);

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api_router)
        .layer(TraceLayer::new_for_http())
}
