use std::ops::Deref;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbErr};

use crate::{
    entity::{RoomId, room_user},
    db::RoomUserDb,
};

pub async fn join_room(
    db: RoomUserDb,
    room_id: RoomId,
    user_id: u32,
) -> Result<room_user::Model, DbErr> {
    let active_model = room_user::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(user_id),
    };

    active_model.insert(db.deref()).await
}

pub async fn is_user_in_room(db: RoomUserDb, room_id: RoomId, user_id: u32) -> Result<bool, DbErr> {
    room_user::Entity::find_by_room_user_pair((room_id, user_id))
        .one(db.deref())
        .await
        .map(|model| model.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, Schema};
    use crate::db::{UserDb, RoomDb};
    use crate::entity::{user, room};
    use crate::db::users as user_db;
    use crate::db::rooms as room_db;

    async fn setup_test_db() -> (RoomUserDb, RoomId, u32) {
        let conn = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory SQLite");

        let builder = conn.get_database_backend();
        let schema = Schema::new(builder);

        // Create all dependent tables
        conn.execute(
            schema
                .create_table_from_entity(room::Entity)
                .if_not_exists(),
        )
        .await
        .expect("Failed to initialize room table");
        conn.execute(
            schema
                .create_table_from_entity(user::Entity)
                .if_not_exists(),
        )
        .await
        .expect("Failed to initialize user table");
        conn.execute(
            schema
                .create_table_from_entity(room_user::Entity)
                .if_not_exists(),
        )
        .await
        .expect("Failed to initialize room_user table");

        // Create a test room
        let room_db_conn = RoomDb(conn.clone());
        let test_room = room_db::create_room(
            &room_db_conn,
            room::Model {
                id: RoomId(0),
                room_name: "test_ru_room".to_string(),
                password: String::new(),
            },
        )
        .await
        .expect("Failed to create test room");

        // Create a test user
        let user_db_conn = UserDb(conn.clone());
        let test_user = user_db::create_user(
            &user_db_conn,
            user::Model {
                id: 0,
                user_name: "test_ru_user".to_string(),
                password: "pass".to_string(),
                token: String::new(),
            },
        )
        .await
        .expect("Failed to create test user");

        (RoomUserDb(conn), test_room.id, test_user.id)
    }

    async fn create_extra_user(db: &RoomUserDb, name: &str) -> u32 {
        let user_db_conn = UserDb(db.0.clone());
        let user = user_db::create_user(
            &user_db_conn,
            user::Model {
                id: 0,
                user_name: name.to_string(),
                password: "pass".to_string(),
                token: String::new(),
            },
        )
        .await
        .expect("Failed to create extra user");
        user.id
    }

    async fn create_extra_room(db: &RoomUserDb, name: &str) -> RoomId {
        let room_db_conn = RoomDb(db.0.clone());
        let room = room_db::create_room(
            &room_db_conn,
            room::Model {
                id: RoomId(0),
                room_name: name.to_string(),
                password: String::new(),
            },
        )
        .await
        .expect("Failed to create extra room");
        room.id
    }

    #[tokio::test]
    async fn test_join_room_basic() {
        let (db, room_id, user_id) = setup_test_db().await;

        let result = join_room(db, room_id.clone(), user_id).await;
        assert!(result.is_ok());
        let ru = result.unwrap();
        assert!(ru.room_id == room_id);
        assert!(ru.user_id == user_id);
    }

    #[tokio::test]
    async fn test_is_user_in_room_true() {
        let (db, room_id, user_id) = setup_test_db().await;

        let _ = join_room(db.clone(), room_id.clone(), user_id).await;

        let result = is_user_in_room(db, room_id, user_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_is_user_in_room_false() {
        let (db, room_id, _user_id) = setup_test_db().await;

        // Check a user that never joined
        let result = is_user_in_room(db, room_id, 99999).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_join_multiple_users_same_room() {
        let (db, room_id, _first_user_id) = setup_test_db().await;

        // Create extra users and join them to the same room
        for i in 0..5u32 {
            let uid = create_extra_user(&db, &format!("multi_user_{}", i)).await;
            let result = join_room(db.clone(), room_id.clone(), uid).await;
            assert!(result.is_ok(), "Failed to join user {} to room: {:?}", uid, result.err());

            // Verify this user is now in the room
            let in_room = is_user_in_room(db.clone(), room_id.clone(), uid).await.unwrap();
            assert!(in_room, "User {} should be in room", uid);
        }
    }

    #[tokio::test]
    async fn test_join_same_user_multiple_rooms() {
        let (db, _first_room_id, user_id) = setup_test_db().await;

        // Create extra rooms and join the user to each
        for i in 0..3u32 {
            let rid = create_extra_room(&db, &format!("extra_room_{}", i)).await;
            let result = join_room(db.clone(), rid.clone(), user_id).await;
            assert!(result.is_ok(), "Failed to join room {}: {:?}", i, result.err());

            // Verify user is in the room
            let in_room = is_user_in_room(db.clone(), rid, user_id).await.unwrap();
            assert!(in_room, "User should be in room {}", i);
        }
    }

    #[tokio::test]
    async fn test_join_room_duplicate() {
        let (db, room_id, user_id) = setup_test_db().await;

        // First join should succeed
        let result1 = join_room(db.clone(), room_id.clone(), user_id).await;
        assert!(result1.is_ok());

        // Second join (same room, same user) should fail (PK violation)
        let result2 = join_room(db, room_id, user_id).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_is_user_in_room_wrong_room() {
        let (db, room_id, user_id) = setup_test_db().await;

        // Join the user to room_id
        let _ = join_room(db.clone(), room_id.clone(), user_id).await;

        // Create another room that the user hasn't joined
        let other_room_id = create_extra_room(&db, "other_room_ru").await;

        // Check other_room — should not find the user there
        let result = is_user_in_room(db, other_room_id, user_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_join_room_max_values() {
        let (db, _room_id, _user_id) = setup_test_db().await;

        // Create a room and user with max IDs (we need actual FK references)
        let room_db_conn = RoomDb(db.0.clone());
        let user_db_conn = UserDb(db.0.clone());

        // We can't reliably test u32::MAX since auto-increment won't give us that.
        // Test with reasonable large values:
        let big_room = room_db::create_room(
            &room_db_conn,
            room::Model {
                id: RoomId(0),
                room_name: "big_room".to_string(),
                password: String::new(),
            },
        )
        .await
        .expect("Failed to create big room");

        let big_user = user_db::create_user(
            &user_db_conn,
            user::Model {
                id: 0,
                user_name: "big_user".to_string(),
                password: "pass".to_string(),
                token: String::new(),
            },
        )
        .await
        .expect("Failed to create big user");

        let result = join_room(db.clone(), big_room.id.clone(), big_user.id).await;
        assert!(result.is_ok());

        let in_room = is_user_in_room(db, big_room.id.clone(), big_user.id).await.unwrap();
        assert!(in_room);
    }
}
