use sea_orm::DeriveEntityModel;
use sea_orm::prelude::*;

use crate::entity::room;
use crate::entity::user;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "room_users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub room_id: u32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: u32,
    #[sea_orm(belongs_to, from = "room_id", to = "id")]
    pub room: Option<room::Entity>,
    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: Option<user::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_user_model_creation() {
        let ru = Model {
            room_id: 1,
            user_id: 42,
        };

        assert_eq!(ru.room_id, 1);
        assert_eq!(ru.user_id, 42);
    }

    #[test]
    fn test_room_user_model_clone() {
        let ru1 = Model {
            room_id: 2,
            user_id: 99,
        };

        let ru2 = ru1.clone();

        assert_eq!(ru1, ru2);
        assert_eq!(ru1.room_id, ru2.room_id);
        assert_eq!(ru1.user_id, ru2.user_id);
    }

    #[test]
    fn test_room_user_model_equality() {
        let ru1 = Model {
            room_id: 3,
            user_id: 7,
        };

        let ru2 = Model {
            room_id: 3,
            user_id: 7,
        };

        let ru3 = Model {
            room_id: 3,
            user_id: 8,
        };

        // 相同 room_id + user_id → 相等
        assert_eq!(ru1, ru2);
        // 不同 user_id → 不等
        assert_ne!(ru1, ru3);
    }

    #[test]
    fn test_room_user_different_room() {
        let ru1 = Model {
            room_id: 1,
            user_id: 100,
        };

        let ru2 = Model {
            room_id: 2,
            user_id: 100,
        };

        // 同一用户在不同房间
        assert_ne!(ru1, ru2);
    }

    #[test]
    fn test_room_user_max_values() {
        let ru = Model {
            room_id: u32::MAX,
            user_id: u32::MAX,
        };

        assert_eq!(ru.room_id, u32::MAX);
        assert_eq!(ru.user_id, u32::MAX);
    }
}
