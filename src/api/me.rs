use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::CookieJar;
use sea_orm::sea_query::value::prelude::serde_json::json;

use crate::db::user;

pub(super) async fn handler_me(
    State(db): State<sea_orm::DatabaseConnection>,
    cookie_jar: CookieJar,
) -> impl IntoResponse {
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    let no_user = (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({
            "message": "请登录",
        })),
    )
        .into_response();

    let Some(token) = cookie_jar.get("token") else {
        return no_user;
    };

    match user::find_by_token(&db, token.value().to_string()).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            axum::Json(json!({
                "message": "验证成功",
                "user_name": user.user_name,
            })),
        )
            .into_response(),
        Ok(None) => no_user,
        Err(_) => db_error,
    }
}
