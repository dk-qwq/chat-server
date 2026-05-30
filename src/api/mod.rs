use axum::{Router, routing::post};

use crate::api::{login::handler_login, register::handler_register};

mod login;
mod register;


pub fn init_api_router() -> Router<sea_orm::DatabaseConnection> {
    Router::new()
        .route("/login", post(handler_login))
        .route("/register", post(handler_register))
}