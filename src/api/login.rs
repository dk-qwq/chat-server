use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use crate::{db::users};
use sea_orm::sea_query::value::prelude::serde_json::json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct LoginRequest {
    user_name: String,
    password: String,
}

pub(super) async fn handler_login(
    State(db): State<sea_orm::DatabaseConnection>,
    cookie_jar: CookieJar,
    Json(LoginRequest {
        user_name,
        password,
    }): Json<LoginRequest>,
) -> impl IntoResponse {
    if user_name.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({
                "message": "用户名或密码不能为空"
            })),
        )
            .into_response();
    }

    let unauthorized_error = (
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({
            "message": "用户不存在或密码错误"
        })),
    )
        .into_response();
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    match users::find_by_user_name(&db, user_name).await {
        Err(_) => db_error,
        Ok(None) => unauthorized_error,
        Ok(Some(user)) => match user.password == password {
            true => (
                StatusCode::OK,
                cookie_jar.add(Cookie::new("token", user.token)),
                axum::Json(json!({
                    "message": "登录成功",
                })),
            )
                .into_response(),
            false => unauthorized_error,
        },
    }
}
