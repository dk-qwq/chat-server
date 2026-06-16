use crate::{entity::message, ws::session::{SessionHandle, SessionId}};

pub enum RoomCommand {
    Join {
        session: SessionHandle
    },
    Leave {
        session_id: SessionId
    },
    ClientMessage {
        message: message::Model
    },
    Stop
}
