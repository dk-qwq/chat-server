use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use sea_orm::sea_query::value::prelude::serde_json;
use serde::{Deserialize, Serialize};

use crate::{db::user, entity::users};

#[derive(Serialize, Deserialize)]
pub(super) struct RegisterRequest {
    user_name: String,
    password: String,
}

pub(super) async fn handler_register(
    State(db): State<sea_orm::DatabaseConnection>,
    Json(RegisterRequest {
        user_name,
        password,
    }): Json<RegisterRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if user_name.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "message": "用户名或密码不能为空"
            })
            .into(),
        );
    }

    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "message": "远程连接错误",
        })
        .into(),
    );

    match user::find_by_user_name(&db, user_name.clone()).await {
        Err(_) => {
            return db_error;
        }
        Ok(Some(_)) => {
            return (
                StatusCode::CONFLICT,
                serde_json::json!({
                        "message": "该用户名已被使用",
                })
                .into(),
            );
        }
        Ok(None) => {}
    }

    match user::create_user(
        &db,
        users::Model {
            id: 0,
            user_name,
            password,
            token: String::new(),
        },
    )
    .await
    {
        Err(_) => db_error,
        Ok(_) => (
            StatusCode::CREATED,
            serde_json::json!(
              {"message": "注册成功"}
            )
            .into(),
        ),
    }
}
