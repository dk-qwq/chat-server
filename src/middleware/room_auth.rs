use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{
    db::room_users::is_user_in_room,
    entity::{RoomId, user},
    db::RoomUserDb,
};

pub async fn locate_room(
    Path(params): Path<HashMap<String, u32>>,
    State(db): State<RoomUserDb>,
    Extension(current_user): Extension<user::Model>,
    mut request: Request,
    next: Next,
) -> Response {
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    let not_room = (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({
            "message": "不正确的房间",
        })),
    )
        .into_response();

    let room_id = RoomId(*params.get("room_id").unwrap_or(&0u32));
    let user_id = current_user.id;

    match is_user_in_room(db, room_id.clone(), user_id).await {
        Err(_) => db_error,
        Ok(false) => not_room,
        Ok(true) => {
            request.extensions_mut().insert(room_id);

            next.run(request).await
        }
    }
}
