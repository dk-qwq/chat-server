use sea_orm::entity::prelude::*;

use crate::entity::{message, user};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    #[sea_orm(unique)]
    pub room_name: String,
    pub password: String,

    #[sea_orm(has_many, via = "room_user")]
    pub users: HasMany<user::Entity>,
    #[sea_orm(has_many)]
    pub messages: HasMany<message::Entity>,
    
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_model_creation() {
        let room = Model {
            id: 1,
            room_name: "general".to_string(),
            password: "secret123".to_string(),
        };

        assert_eq!(room.id, 1);
        assert_eq!(room.room_name, "general");
        assert_eq!(room.password, "secret123");
    }

    #[test]
    fn test_room_model_clone() {
        let room1 = Model {
            id: 2,
            room_name: "lobby".to_string(),
            password: "pass".to_string(),
        };

        let room2 = room1.clone();

        assert_eq!(room1, room2);
        assert_eq!(room1.room_name, room2.room_name);
        assert_eq!(room1.password, room2.password);
    }

    #[test]
    fn test_room_model_equality() {
        let room1 = Model {
            id: 3,
            room_name: "same".to_string(),
            password: "same_pass".to_string(),
        };

        let room2 = Model {
            id: 3,
            room_name: "same".to_string(),
            password: "same_pass".to_string(),
        };

        let room3 = Model {
            id: 4,
            room_name: "different".to_string(),
            password: "different_pass".to_string(),
        };

        assert_eq!(room1, room2);
        assert_ne!(room1, room3);
    }

    #[test]
    fn test_room_model_special_characters() {
        let room = Model {
            id: 5,
            room_name: "中文房间".to_string(),
            password: "!@#$%^&*()".to_string(),
        };

        assert_eq!(room.room_name, "中文房间");
        assert_eq!(room.password, "!@#$%^&*()");
    }

    #[test]
    fn test_room_model_empty_password() {
        let room = Model {
            id: 6,
            room_name: "no_password_room".to_string(),
            password: String::new(),
        };

        assert!(room.password.is_empty());
    }

    #[test]
    fn test_room_model_long_strings() {
        let long_name = "r".repeat(255);
        let long_password = "p".repeat(255);
        let room = Model {
            id: 7,
            room_name: long_name.clone(),
            password: long_password.clone(),
        };

        assert_eq!(room.room_name.len(), 255);
        assert_eq!(room.password.len(), 255);
    }
}