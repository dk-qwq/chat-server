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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new_generates_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        // Two randomly generated session IDs should be different
        assert!(id1 != id2);
    }

    #[test]
    fn test_session_id_clone_equality() {
        let id1 = SessionId::new();
        let id2 = id1.clone();
        assert!(id1 == id2);
    }

    #[test]
    fn test_session_id_default() {
        let id1 = SessionId::default();
        let id2 = SessionId::default();
        // Each default() call generates a new random session id
        assert!(id1 != id2);
    }

    #[test]
    fn test_session_id_format() {
        let id = SessionId::new();
        // SessionId is base64 URL-safe encoded, 32 bytes → ~43 chars (no padding)
        assert!(id.0.len() >= 40);
        // Should be URL-safe (no +, /, = characters)
        assert!(!id.0.contains('+'));
        assert!(!id.0.contains('/'));
        assert!(!id.0.contains('='));
    }

    #[test]
    fn test_session_id_not_empty() {
        let id = SessionId::new();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_session_handle_creation() {
        let (tx, _rx) = mpsc::channel::<message::Model>(1);
        let handle = SessionHandle::new("alice".to_string(), tx);

        assert!(handle.user_name == "alice");
        assert!(!handle.session_id.0.is_empty());
    }

    #[test]
    fn test_session_handle_unique_session_ids() {
        let (tx1, _rx1) = mpsc::channel::<message::Model>(1);
        let (tx2, _rx2) = mpsc::channel::<message::Model>(1);

        let handle1 = SessionHandle::new("user1".to_string(), tx1);
        let handle2 = SessionHandle::new("user2".to_string(), tx2);

        // Each handle should have a unique session ID
        assert!(handle1.session_id != handle2.session_id);
    }
}
