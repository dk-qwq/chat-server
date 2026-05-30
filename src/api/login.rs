use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chat_server::db::user;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct LoginRequest {
    user_id: String,
    password: String,
}

#[derive(Serialize)]
pub(super) struct AuthBody {
    token: String,
}

pub(super) async fn handler_login(
    State(db): State<sea_orm::DatabaseConnection>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthBody>, StatusCode> {
    match user::find_by_user_id(&db, payload.user_id).await {
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
        Ok(None) => Err(StatusCode::UNAUTHORIZED),
        Ok(Some(user)) => {
            if user.password == payload.password {
                Ok(Json(AuthBody { token: user.token }))
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}
