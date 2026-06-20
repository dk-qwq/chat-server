use sea_orm::DeriveValueType;

pub mod message;
pub mod room;
pub mod room_user;
pub mod user;

#[derive(Clone, Debug, PartialEq, Eq, DeriveValueType, Hash)]
pub struct RoomId(pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_id_creation() {
        let id = RoomId(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_room_id_clone() {
        let id1 = RoomId(1);
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_room_id_equality() {
        assert_eq!(RoomId(1), RoomId(1));
        assert_ne!(RoomId(1), RoomId(2));
    }

    #[test]
    fn test_room_id_zero() {
        let id = RoomId(0);
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_room_id_max() {
        let id = RoomId(u32::MAX);
        assert_eq!(id.0, u32::MAX);
    }

    #[test]
    fn test_room_id_debug_format() {
        let id = RoomId(5);
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_room_id_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id1 = RoomId(10);
        let id2 = RoomId(10);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        id1.hash(&mut hasher1);
        id2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }
}
