use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::{
    api::init_api_router,
    db::{MessageDb, RoomDb, RoomUserDb, UserDb},
    state::AppState,
    ws::hub::Chathub,
};

mod api;
mod config;
mod db;
mod entity;
mod middleware;
mod state;
mod utils;
mod ws;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let user_db = UserDb::connect("sqlite://users.sqlite?mode=rwc").await;
    let message_db = MessageDb::connect("sqlite://messages.sqlite?mode=rwc").await;
    let room_db = RoomDb::connect("sqlite://rooms.sqlite?mode=rwc").await;
    let room_users_db = RoomUserDb::connect("sqlite://room_user.sqlite?mode=rwc").await;

    let chathub = Chathub::new(room_db.clone(), message_db.clone()).await;

    let app_state = AppState {
        user_db,
        room_db,
        room_users_db,
        message_db,
        chathub,
    };

    let api_router = init_api_router(app_state);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api_router)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}
