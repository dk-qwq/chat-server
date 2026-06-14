use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast::Sender;

use axum::extract::ws::Message as WsMessage;

use crate::entity::{message::Message, users};

pub(super) async fn handler_ws(
    ws: WebSocketUpgrade,
    State(tx): State<Sender<String>>,
    Extension(current_user): Extension<users::Model>,
) -> Response {
    ws.on_upgrade(|socket| handler_socket(socket, tx, current_user))
}

async fn handler_socket(socket: WebSocket, tx: Sender<String>, current_user: users::Model) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = tx.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if sender.send(WsMessage::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(WsMessage::Text(message))) = receiver.next().await {
            if let Ok(mut message) = serde_json::from_str::<Message>(message.as_str()) {
                if message.user_name != current_user.user_name {
                    continue;
                }

                message.timestamp = Utc::now();

                // TODO: fix/modify id in db

                let json_str = match serde_json::to_string(&message) {
                    Ok(json_str) => json_str,
                    Err(_) => continue,
                };

                if tx.send(json_str).is_err() {
                    break;
                }
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
}
