#![allow(dead_code)]

use sea_orm::entity::prelude::*;

use crate::entity::{message, room};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    #[sea_orm(unique)]
    pub user_name: String,
    pub password: String,
    #[sea_orm(unique)]
    pub token: String,

    #[sea_orm(has_many, via = "room_user")]
    pub rooms: HasMany<room::Entity>,
    #[sea_orm(has_many)]
    pub messages: HasMany<message::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_model_creation() {
        let user = Model {
            id: 1,
            user_name: "testuser".to_string(),
            password: "password123".to_string(),
            token: "token_abc123".to_string(),
        };

        assert_eq!(user.id, 1);
        assert_eq!(user.user_name, "testuser");
        assert_eq!(user.password, "password123");
        assert_eq!(user.token, "token_abc123");
    }

    #[test]
    fn test_user_model_clone() {
        let user1 = Model {
            id: 2,
            user_name: "user2".to_string(),
            password: "pass2".to_string(),
            token: "token2".to_string(),
        };

        let user2 = user1.clone();

        assert_eq!(user1, user2);
        assert_eq!(user1.user_name, user2.user_name);
    }

    #[test]
    fn test_user_model_equality() {
        let user1 = Model {
            id: 3,
            user_name: "user3".to_string(),
            password: "pass3".to_string(),
            token: "token3".to_string(),
        };

        let user2 = Model {
            id: 3,
            user_name: "user3".to_string(),
            password: "pass3".to_string(),
            token: "token3".to_string(),
        };

        let user3 = Model {
            id: 4,
            user_name: "user4".to_string(),
            password: "pass4".to_string(),
            token: "token4".to_string(),
        };

        assert_eq!(user1, user2);
        assert_ne!(user1, user3);
    }

    #[test]
    fn test_user_model_special_characters() {
        let user = Model {
            id: 5,
            user_name: "user@domain.com".to_string(),
            password: "p@ssw0rd!#$%".to_string(),
            token: "ABCDefg123XYZ".to_string(),
        };

        assert_eq!(user.user_name, "user@domain.com");
        assert_eq!(user.password, "p@ssw0rd!#$%");
    }

    #[test]
    fn test_user_model_empty_strings() {
        let user = Model {
            id: 6,
            user_name: String::new(),
            password: String::new(),
            token: String::new(),
        };

        assert!(user.user_name.is_empty());
        assert!(user.password.is_empty());
        assert!(user.token.is_empty());
    }

    #[test]
    fn test_user_model_long_strings() {
        let long_username = "a".repeat(1000);
        let long_password = "b".repeat(2000);
        let long_token = "c".repeat(256);

        let user = Model {
            id: 7,
            user_name: long_username.clone(),
            password: long_password.clone(),
            token: long_token.clone(),
        };

        assert_eq!(user.user_name, long_username);
        assert_eq!(user.password, long_password);
        assert_eq!(user.token, long_token);
    }
}
