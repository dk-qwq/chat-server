use std::collections::HashMap;

use futures::future::join_all;
use tokio::sync::mpsc;
use tracing::error;

use crate::{
    db::messages,
    entity::message,
    state::MessageDb,
    ws::{
        protocol::RoomCommand,
        session::{SessionHandle, SessionId},
    },
};

pub struct RoomActor {
    tx: mpsc::Sender<RoomCommand>,
    rx: mpsc::Receiver<RoomCommand>,
    members: HashMap<SessionId, SessionHandle>,
    db: MessageDb,
}

impl RoomActor {
    pub fn new(buffer: usize, db: MessageDb) -> Self {
        let (tx, rx) = mpsc::channel::<RoomCommand>(buffer);
        Self {
            tx,
            rx,
            members: HashMap::default(),
            db,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<RoomCommand> {
        self.tx.clone()
    }

    pub async fn run(mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                RoomCommand::Join { session } => {
                    self.members.insert(session.session_id.clone(), session);
                }
                RoomCommand::Leave { session_id } => {
                    self.members.remove(&session_id);
                }
                RoomCommand::ClientMessage { message } => {
                    if let Err(err) = self.handler_message(message).await {
                        error!("insert message into db error: {err}");
                    }
                }
                RoomCommand::Stop => {
                    break;
                }
            }
        }
    }

    async fn handler_message(&self, message: message::Model) -> Result<(), sea_orm::DbErr> {
        let message = messages::insert_message(&self.db, message).await?;

        let futures = self.members.values().map(|handle| {
            let tx = handle.tx.clone();
            let value = message.clone();
            async move {
                let _ = tx.send(value).await;
            }
        });

        join_all(futures).await;
        Ok(())
    }
}
