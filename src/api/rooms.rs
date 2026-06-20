use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::Response,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::RoomUserDb,
    db::room_users::join_room,
    entity::{RoomId, room, user},
    ws::hub::{Chathub, CreateRoomError},
};

#[derive(Serialize, Deserialize)]
pub(super) struct CreateRoomRequest {
    room_name: String,
    password: String,
}

use crate::utils::json_resp;

pub(super) async fn handler_create_room(
    State(chathub): State<Chathub>,
    State(db): State<RoomUserDb>,
    Extension(user): Extension<user::Model>,
    Query(CreateRoomRequest {
        room_name,
        password,
    }): Query<CreateRoomRequest>,
) -> Result<Response, Response> {
    if room_name.is_empty() {
        return Err(json_resp(StatusCode::BAD_REQUEST, "房间名不能为空"));
    }

    let room = room::Model {
        id: RoomId(0),
        room_name,
        password,
    };

    let room_id = chathub.create_room(room).await.map_err(|err| match err {
        CreateRoomError::DuplicateRoomName => json_resp(StatusCode::CONFLICT, "房间名重复"),
        CreateRoomError::Db(_) => json_resp(StatusCode::SERVICE_UNAVAILABLE, "远程连接错误"),
    })?;

    join_room(db, room_id, user.id)
        .await
        .map_err(|_| json_resp(StatusCode::SERVICE_UNAVAILABLE, "远程连接错误"))?;

    Ok(json_resp(StatusCode::OK, "创建房间成功"))
}

pub(super) async fn handler_get_rooms(// Extension(user): Extension<user::Model>,
    // State(chathub): State<Chathub>,
) -> Response {
    json_resp(StatusCode::IM_A_TEAPOT, "还没实现呢")
}
