use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
};

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use axum::extract::ws::Message as WsMessage;
use tracing::{error, info, warn};

use crate::{
    entity::{message, user},
    ws::{hub::Chathub, protocol::RoomCommand, session::SessionHandle},
};

pub(super) async fn handler_ws(
    ws: WebSocketUpgrade,
    State(chathub): State<Chathub>,
    Extension(current_user): Extension<user::Model>,
) -> Response {
    ws.on_upgrade(|socket| handler_socket(socket, chathub.global_room_tx, current_user))
}

async fn handler_socket(
    socket: WebSocket,
    room_tx: mpsc::Sender<RoomCommand>,
    current_user: user::Model,
) {
    let username = current_user.user_name.clone();

    const USER_MESSAGE_BUFFER_SIZE: usize = 20;
    let (tx, mut rx) = mpsc::channel::<message::Model>(USER_MESSAGE_BUFFER_SIZE);
    let session_handle = SessionHandle::new(username.clone(), tx);
    let session_id = session_handle.session_id.clone();

    if let Err(err) = room_tx
        .send(RoomCommand::Join {
            session: session_handle,
        })
        .await
    {
        error!("send join message error: {err}");
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let json_str = match serde_json::to_string(&message) {
                Ok(json) => json,
                Err(err) => {
                    warn!("serialize message failed, error: {err}");
                    continue;
                }
            };

            if sender.send(WsMessage::Text(json_str.into())).await.is_err() {
                break;
            }
        }
    });

    let user_name = current_user.user_name;
    let recv_room_tx = room_tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(WsMessage::Text(raw_message))) = receiver.next().await {
            let message = match serde_json::from_str::<message::Model>(raw_message.as_str()) {
                Ok(message) => message,
                Err(err) => {
                    info!("{raw_message}");
                    info!("failed to deserialize message with error: {err}");
                    continue;
                }
            };

            if message.user_name != user_name {
                continue;
            }

            if let Err(err) = recv_room_tx
                .send(RoomCommand::ClientMessage { message })
                .await
            {
                error!("send message error: {err}");
            }
        }
    });

    tokio::select! {
        _ = (& mut send_task) => {
            recv_task.abort();
        },
        _ = (& mut recv_task) => {
            send_task.abort();
        }
    }

    if let Err(err) = room_tx.send(RoomCommand::Leave { session_id }).await {
        error!("send leave message error: {err}");
    }
}
