pub mod api;
pub mod db;
pub mod entity;

use axum::{Router, routing::get};
use sea_orm::Database;
use tower_http::trace::TraceLayer;

/// 构建测试用 App（使用内存数据库）
pub async fn build_test_app() -> Router {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    db::init_user_table(&db).await;

    let api_router = api::init_api_router().with_state(db);

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api_router)
        .layer(TraceLayer::new_for_http())
}
