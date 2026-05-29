use axum::{
    Router,
    routing::get
};

mod db;
mod entity;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(|| async {"OK"}));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}