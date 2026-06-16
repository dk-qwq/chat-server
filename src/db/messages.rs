use std::ops::Deref;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

use sea_orm::Set;

use crate::state::MessageDb;
use crate::entity::message;

pub async fn insert_message(db: &MessageDb, form: message::Model) -> Result<message::Model, DbErr> {
    let active_model = message::ActiveModel {
        user_name: Set(form.user_name),
        content: Set(form.content),
        timestamp: Set(Utc::now()),
        ..Default::default()
    };

    active_model.insert(db.deref()).await
}

pub async fn latest_message_id(db: &MessageDb) -> Result<Option<u32>, DbErr> {
    message::Entity::find()
        .order_by_id_desc()
        .one(db.deref())
        .await
        .map(|opt| opt.map(|msg| msg.id))
}

pub async fn list_message_before(
    db: &MessageDb,
    before_id: u32,
    limit: u64,
) -> Result<Vec<message::Model>, DbErr> {
    message::Entity::find()
        .filter(message::Column::Id.lte(before_id))
        .order_by_id_desc()
        .limit(limit)
        .all(db.deref())
        .await
}

pub async fn list_message_after(
    db: &MessageDb,
    after_id: u32,
    limit: u64,
) -> Result<Vec<message::Model>, DbErr> {
    message::Entity::find()
        .filter(message::Column::Id.gte(after_id))
        .order_by_id_asc()
        .limit(limit)
        .all(db.deref())
        .await
}

pub async fn list_message(
    db: &MessageDb,
    limit: u64,
) -> Result<Vec<message::Model>, DbErr> {
    message::Entity::find()
        .order_by_id_desc()
        .limit(limit)
        .all(db.deref())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, ConnectionTrait};

    async fn setup_test_db() -> MessageDb {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory SQLite");
        let message_db = MessageDb(db);
        
        // Initialize table
        let builder = message_db.get_database_backend();
        let schema = sea_orm::Schema::new(builder);
        message_db
            .execute(
                schema
                    .create_table_from_entity(message::Entity)
                    .if_not_exists(),
            )
            .await
            .expect("Failed to initialize message table");
        
        message_db
    }

    #[tokio::test]
    async fn test_insert_message_basic() {
        let db = setup_test_db().await;
        let msg = message::Model {
            id: 0,
            user_name: "test_user".to_string(),
            content: "test content".to_string(),
            timestamp: Utc::now(),
        };

        let result = insert_message(&db, msg.clone()).await;

        assert!(result.is_ok());
        let inserted = result.unwrap();
        assert_eq!(inserted.user_name, "test_user");
        assert_eq!(inserted.content, "test content");
        assert!(inserted.id > 0);
    }

    #[tokio::test]
    async fn test_insert_message_with_special_content() {
        let db = setup_test_db().await;
        let msg = message::Model {
            id: 0,
            user_name: "user@123".to_string(),
            content: "Message with 中文, emoji 😊 and \n newlines!".to_string(),
            timestamp: Utc::now(),
        };

        let result = insert_message(&db, msg.clone()).await;

        assert!(result.is_ok());
        let inserted = result.unwrap();
        assert_eq!(inserted.user_name, "user@123");
        assert_eq!(inserted.content, "Message with 中文, emoji 😊 and \n newlines!");
    }

    #[tokio::test]
    async fn test_latest_message_id_empty() {
        let db = setup_test_db().await;

        let result = latest_message_id(&db).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_latest_message_id_single_message() {
        let db = setup_test_db().await;
        let msg = message::Model {
            id: 0,
            user_name: "user1".to_string(),
            content: "msg1".to_string(),
            timestamp: Utc::now(),
        };
        let _ = insert_message(&db, msg).await;

        let result = latest_message_id(&db).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(1));
    }

    #[tokio::test]
    async fn test_latest_message_id_multiple_messages() {
        let db = setup_test_db().await;
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = latest_message_id(&db).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(5));
    }

    #[tokio::test]
    async fn test_list_message_before_empty_result() {
        let db = setup_test_db().await;
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_before(&db, 0, 10).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_message_before_full_range() {
        let db = setup_test_db().await;
        for i in 1..=3 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_before(&db, 10, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        // Verify descending order
        assert_eq!(messages[0].id, 3);
        assert_eq!(messages[1].id, 2);
        assert_eq!(messages[2].id, 1);
    }

    #[tokio::test]
    async fn test_list_message_before_partial_range() {
        let db = setup_test_db().await;
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_before(&db, 3, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        // Should include ids 1, 2, 3 in descending order
        assert!(messages.iter().all(|m| m.id <= 3));
    }

    #[tokio::test]
    async fn test_list_message_before_respects_limit() {
        let db = setup_test_db().await;
        for i in 1..=10 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_before(&db, 100, 3).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        // Should get the 3 highest ids (10, 9, 8)
        assert_eq!(messages[0].id, 10);
        assert_eq!(messages[1].id, 9);
        assert_eq!(messages[2].id, 8);
    }

    #[tokio::test]
    async fn test_list_message_after_empty_result() {
        let db = setup_test_db().await;
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_after(&db, 100, 10).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_message_after_full_range() {
        let db = setup_test_db().await;
        for i in 1..=3 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_after(&db, 1, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        // Verify ascending order
        assert_eq!(messages[0].id, 1);
        assert_eq!(messages[1].id, 2);
        assert_eq!(messages[2].id, 3);
    }

    #[tokio::test]
    async fn test_list_message_after_partial_range() {
        let db = setup_test_db().await;
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_after(&db, 3, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3); // ids 3, 4, 5
        assert!(messages.iter().all(|m| m.id >= 3));
    }

    #[tokio::test]
    async fn test_list_message_after_respects_limit() {
        let db = setup_test_db().await;
        for i in 1..=10 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message_after(&db, 1, 3).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        // Should get the 3 lowest ids (1, 2, 3)
        assert_eq!(messages[0].id, 1);
        assert_eq!(messages[1].id, 2);
        assert_eq!(messages[2].id, 3);
    }

    #[tokio::test]
    async fn test_message_content_preserved_exactly() {
        let db = setup_test_db().await;
        let original_content = "Line 1\nLine 2\tTabbed\r\nSpecial: !@#$%^&*()";
        let msg = message::Model {
            id: 0,
            user_name: "tester".to_string(),
            content: original_content.to_string(),
            timestamp: Utc::now(),
        };

        let inserted = insert_message(&db, msg).await.unwrap();
        let retrieved = list_message_after(&db, inserted.id, 1).await.unwrap();

        assert_eq!(retrieved[0].content, original_content);
    }

    #[tokio::test]
    async fn test_list_message_empty_table() {
        let db = setup_test_db().await;

        let result = list_message(&db, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_list_message_basic() {
        let db = setup_test_db().await;
        
        // 插入 5 条消息
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message(&db, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 5);
        
        // 验证倒序排列（最新的消息在前）
        assert_eq!(messages[0].id, 5);
        assert_eq!(messages[1].id, 4);
        assert_eq!(messages[2].id, 3);
        assert_eq!(messages[3].id, 2);
        assert_eq!(messages[4].id, 1);
    }

    #[tokio::test]
    async fn test_list_message_respects_limit() {
        let db = setup_test_db().await;
        
        // 插入 10 条消息
        for i in 1..=10 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message(&db, 3).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        
        // 应该返回最新的 3 条消息 (10, 9, 8)
        assert_eq!(messages[0].id, 10);
        assert_eq!(messages[1].id, 9);
        assert_eq!(messages[2].id, 8);
    }

    #[tokio::test]
    async fn test_list_message_ordering_descending() {
        let db = setup_test_db().await;
        
        // 插入消息
        for i in 1..=7 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message(&db, 100).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        
        // 验证严格倒序（ID 逐个递减）
        for i in 0..messages.len() - 1 {
            assert!(messages[i].id > messages[i + 1].id);
        }
    }

    #[tokio::test]
    async fn test_list_message_limit_one() {
        let db = setup_test_db().await;
        
        // 插入多条消息
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message(&db, 1).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        // 应该返回最新消息
        assert_eq!(messages[0].id, 5);
    }

    #[tokio::test]
    async fn test_list_message_limit_larger_than_data() {
        let db = setup_test_db().await;
        
        // 插入 3 条消息
        for i in 1..=3 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        // 请求超过数据量的 limit
        let result = list_message(&db, 100).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        // 应该返回所有 3 条消息，而不是 100 条
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_list_message_consistency_with_latest_id() {
        let db = setup_test_db().await;
        
        // 插入 5 条消息
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        let latest_id = latest_message_id(&db).await.unwrap().unwrap();
        let list_result = list_message(&db, 1).await.unwrap();

        // 最新 ID 应该与 list_message(1) 的第一条消息 ID 相同
        assert_eq!(latest_id, list_result[0].id);
    }

    #[tokio::test]
    async fn test_list_message_multiple_calls_consistency() {
        let db = setup_test_db().await;
        
        // 插入 10 条消息
        for i in 1..=10 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: Utc::now(),
            };
            let _ = insert_message(&db, msg).await;
        }

        // 多次调用应该返回相同结果
        let result1 = list_message(&db, 5).await.unwrap();
        let result2 = list_message(&db, 5).await.unwrap();

        assert_eq!(result1.len(), result2.len());
        for i in 0..result1.len() {
            assert_eq!(result1[i].id, result2[i].id);
            assert_eq!(result1[i].user_name, result2[i].user_name);
            assert_eq!(result1[i].content, result2[i].content);
        }
    }

    #[tokio::test]
    async fn test_list_message_with_identical_timestamps() {
        let db = setup_test_db().await;
        
        let now = Utc::now();
        
        // 插入多条消息（时间戳相同）
        for i in 1..=5 {
            let msg = message::Model {
                id: 0,
                user_name: format!("user{}", i),
                content: format!("msg{}", i),
                timestamp: now,
            };
            let _ = insert_message(&db, msg).await;
        }

        let result = list_message(&db, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        // 即使时间戳相同，也应该按 ID 倒序
        for i in 0..messages.len() - 1 {
            assert!(messages[i].id > messages[i + 1].id);
        }
    }

    #[tokio::test]
    async fn test_list_message_returns_correct_fields() {
        let db = setup_test_db().await;
        
        let msg = message::Model {
            id: 0,
            user_name: "alice".to_string(),
            content: "Test message with特殊字符 and emoji 😊".to_string(),
            timestamp: Utc::now(),
        };
        let _ = insert_message(&db, msg).await;

        let result = list_message(&db, 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        
        let retrieved = &messages[0];
        assert_eq!(retrieved.user_name, "alice");
        assert_eq!(retrieved.content, "Test message with特殊字符 and emoji 😊");
        assert!(retrieved.id > 0);
    }
}
