use std::ops::Deref;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbErr, EntityTrait};

use crate::{
    entity::{RoomId, room},
    db::RoomDb,
};

pub async fn create_room(db: &RoomDb, form: room::Model) -> Result<room::Model, DbErr> {
    let active_model = room::ActiveModel {
        room_name: Set(form.room_name),
        password: Set(form.password),
        ..Default::default()
    };

    active_model.insert(db.deref()).await
}

pub async fn find_by_id(db: &RoomDb, id: RoomId) -> Result<Option<room::Model>, DbErr> {
    room::Entity::find_by_id(id).one(db.deref()).await
}

pub async fn find_by_room_name(
    db: &RoomDb,
    room_name: String,
) -> Result<Option<room::Model>, DbErr> {
    room::Entity::find_by_room_name(room_name)
        .one(db.deref())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, Schema};

    async fn setup_test_db() -> RoomDb {
        let conn = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory SQLite");
        let room_db = RoomDb(conn.clone());

        let builder = room_db.get_database_backend();
        let schema = Schema::new(builder);
        room_db
            .execute(
                schema
                    .create_table_from_entity(room::Entity)
                    .if_not_exists(),
            )
            .await
            .expect("Failed to initialize room table");

        room_db
    }

    #[tokio::test]
    async fn test_create_room_basic() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "lobby".to_string(),
            password: "secret".to_string(),
        };

        let result = create_room(&db, room).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.room_name, "lobby");
        assert_eq!(created.password, "secret");
        assert!(created.id.0 > 0);
    }

    #[tokio::test]
    async fn test_create_room_empty_password() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "no_pass_room".to_string(),
            password: String::new(),
        };

        let result = create_room(&db, room).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.room_name, "no_pass_room");
        assert!(created.password.is_empty());
    }

    #[tokio::test]
    async fn test_create_room_special_characters() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "中文房间".to_string(),
            password: "!@#$%^&*()".to_string(),
        };

        let result = create_room(&db, room).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.room_name, "中文房间");
        assert_eq!(created.password, "!@#$%^&*()");
    }

    #[tokio::test]
    async fn test_create_room_duplicate_name() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "unique_room".to_string(),
            password: "pass1".to_string(),
        };
        let _ = create_room(&db, room).await.unwrap();

        let dup_room = room::Model {
            id: RoomId(0),
            room_name: "unique_room".to_string(),
            password: "pass2".to_string(),
        };
        let result = create_room(&db, dup_room).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_id_exists() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "findable".to_string(),
            password: "pass".to_string(),
        };
        let created = create_room(&db, room).await.unwrap();

        let result = find_by_id(&db, created.id.clone()).await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_room = found.unwrap();
        assert_eq!(found_room.id, created.id);
        assert_eq!(found_room.room_name, "findable");
    }

    #[tokio::test]
    async fn test_find_by_id_not_exists() {
        let db = setup_test_db().await;

        let result = find_by_id(&db, RoomId(9999)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_find_by_room_name_exists() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "searchable".to_string(),
            password: "pass".to_string(),
        };
        let _ = create_room(&db, room).await.unwrap();

        let result = find_by_room_name(&db, "searchable".to_string()).await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().room_name, "searchable");
    }

    #[tokio::test]
    async fn test_find_by_room_name_not_exists() {
        let db = setup_test_db().await;

        let result = find_by_room_name(&db, "nonexistent_room".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_create_multiple_rooms() {
        let db = setup_test_db().await;
        for i in 0..5 {
            let room = room::Model {
                id: RoomId(0),
                room_name: format!("room_{}", i),
                password: format!("pass_{}", i),
            };
            let result = create_room(&db, room).await;
            assert!(result.is_ok(), "Failed to create room {}", i);
        }

        // Verify all rooms can be found
        for i in 0..5 {
            let result = find_by_room_name(&db, format!("room_{}", i)).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_some(), "room_{} should exist", i);
        }
    }

    #[tokio::test]
    async fn test_find_by_room_name_case_sensitive() {
        let db = setup_test_db().await;
        let room = room::Model {
            id: RoomId(0),
            room_name: "CaseSensitive".to_string(),
            password: "pass".to_string(),
        };
        let _ = create_room(&db, room).await.unwrap();

        // Different case should not match
        let result = find_by_room_name(&db, "casesensitive".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
}