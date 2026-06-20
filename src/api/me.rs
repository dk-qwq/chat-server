use axum::{Extension, http::StatusCode, response::IntoResponse};
use sea_orm::sea_query::value::prelude::serde_json::json;

use crate::entity::user;

pub(super) async fn handler_me(
    Extension(current_user): Extension<user::Model>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(json!({
            "message": "验证成功",
            "user_name": current_user.user_name,
        })),
    )
        .into_response()
}
