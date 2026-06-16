use tokio::sync::mpsc;

use crate::ws::protocol::RoomCommand;

#[derive(Clone)]
pub struct Chathub {
    pub global_room_tx: mpsc::Sender<RoomCommand>,
}

impl Chathub {
    pub fn new(tx: mpsc::Sender<RoomCommand>) -> Self {
        Chathub { global_room_tx: tx }
    }
}
