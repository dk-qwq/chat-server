use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRng, rngs::SysRng};
use tokio::sync::mpsc;

use crate::entity::message;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 32];
        SysRng.try_fill_bytes(&mut bytes).unwrap();
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionHandle {
    pub session_id: SessionId,
    pub user_name: String,
    pub tx: mpsc::Sender<message::Model>,
}

impl SessionHandle {
    pub fn new(user_name: String, tx: mpsc::Sender<message::Model>) -> Self {
        Self {
            session_id: SessionId::default(),
            user_name,
            tx,
        }
    }
}
