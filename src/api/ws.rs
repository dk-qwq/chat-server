use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast::Sender;

use axum::extract::ws::Message as WsMessage;
use tracing::debug;

use crate::entity::{message::Message, users};

pub(super) async fn handler_ws(
    ws: WebSocketUpgrade,
    State(tx): State<Sender<String>>,
    Extension(current_user): Extension<users::Model>,
) -> Response {
    ws.on_upgrade(|socket| handler_socket(socket, tx, current_user))
}

async fn handler_socket(socket: WebSocket, tx: Sender<String>, current_user: users::Model) {
    let username = current_user.user_name.clone();
    debug!("连接成功, user: {}", username);

    let (mut sender, mut receiver) = socket.split();

    let mut rx = tx.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            debug!("message send: {}", message);
            if sender.send(WsMessage::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(WsMessage::Text(raw_message))) = receiver.next().await {

            debug!("raw_message recv: {}", raw_message);

            let mut message = match serde_json::from_str::<Message>(raw_message.as_str()) {
                Ok(message) => message,
                Err(e) => {
                    debug!("failed to deserialize message with error: {}", e);
                    continue;
                }
            };

            debug!("parsing message: user_name: {}", message.user_name);

            if message.user_name != current_user.user_name {
                continue;
            }

            message.timestamp = Utc::now();

            // TODO: fix/modify id in db

            let json_str = match serde_json::to_string(&message) {
                Ok(json_str) => json_str,
                Err(_) => continue,
            };

            debug!("try send {} to broadcast", json_str);

            if tx.send(json_str).is_err() {
                break;
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

    debug!("断开连接, user: {}", username);
}
