use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;

use crate::{db::messages, state::MessageDb};

#[derive(Deserialize)]
pub(super) struct MessagesQuery {
    before_id: Option<u32>,
    after_id: Option<u32>,
    limit: Option<u32>,
}

pub(super) async fn get_message(
    Query(params): Query<MessagesQuery>,
    State(db): State<MessageDb>,
) -> impl IntoResponse {
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    const LIMIT: u32 = 50;
    let limit = params
        .limit
        .map(|v| std::cmp::Ord::min(v, LIMIT))
        .unwrap_or(LIMIT);

    if params.before_id.is_some() && params.after_id.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({
                "message": "before_id 和 after_id 不能同时存在"
            })),
        )
            .into_response();
    }

    let res = if let Some(before_id) = params.before_id {
        messages::list_message_before(&db, before_id, limit as u64).await
    } else if let Some(after_id) = params.after_id {
        messages::list_message_after(&db, after_id, limit as u64).await
    } else {
        messages::list_message(&db, limit as u64).await
    };

    match res {
        Err(_) => db_error,
        Ok(vec) => (
            StatusCode::OK, 
            axum::Json(json!({
                "messages": vec,
            })),
        ).into_response(),
    }
}

pub(super) async fn latest_message_id(State(db): State<MessageDb>) -> impl IntoResponse {
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    let id = match messages::latest_message_id(&db).await {
        Err(_) => return db_error,
        Ok(Some(id)) => id,
        Ok(None) => 1,
    };

    (
        StatusCode::OK,
        axum::Json(json!({
            "id": id,
        })),
    )
        .into_response()
}
