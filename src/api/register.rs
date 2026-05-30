use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{db::user, entity::users};

#[derive(Serialize, Deserialize)]
pub(super) struct RegisterRequest {
    user_name: String,
    password: String,
}

#[derive(Serialize)]
pub(super) struct AuthBody {
    user_id: String,
}

fn gen_id() -> String {
    rand::rng().random::<i128>().to_string()
}

pub(super) async fn handler_register(
    State(db): State<sea_orm::DatabaseConnection>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthBody>, StatusCode> {
    match user::create_user(&db, users::Model{
        id: 0,
        user_id: gen_id(),
        user_name: payload.user_name,
        password: payload.password,
        token: String::new()
    }).await {
        Err(_) => {
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
        Ok(user) => {
            Ok(Json(AuthBody{
                user_id: user.user_id
            }))
        }
    }
}