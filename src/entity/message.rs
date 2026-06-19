use chrono::Utc;

use serde::Deserialize;
use serde::Serialize;

use sea_orm::entity::prelude::*;

use chrono::DateTime;

use crate::entity::room;
use crate::entity::user;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "messages")]
#[derive(Serialize, Deserialize)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    pub user_name: String,
    pub content: String,

    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    pub room_id: Option<u32>,
    #[sea_orm(belongs_to, from = "room_id", to = "id")]
    #[serde(skip)]
    pub room: HasOne<room::Entity>,

    pub user_id: Option<u32>,
    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    #[serde(skip)]
    pub user: HasOne<user::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_message_model_creation() {
        let now = Utc::now();
        let msg = Model {
            id: 1,
            user_name: "alice".to_string(),
            content: "Hello, World!".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert_eq!(msg.id, 1);
        assert_eq!(msg.user_name, "alice");
        assert_eq!(msg.content, "Hello, World!");
        assert_eq!(msg.timestamp, now);
    }

    #[test]
    fn test_message_model_clone() {
        let now = Utc::now();
        let msg1 = Model {
            id: 2,
            user_name: "bob".to_string(),
            content: "Test message".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        let msg2 = msg1.clone();

        assert_eq!(msg1, msg2);
        assert_eq!(msg1.content, msg2.content);
    }

    #[test]
    fn test_message_model_equality() {
        let now = Utc::now();
        let msg1 = Model {
            id: 3,
            user_name: "charlie".to_string(),
            content: "Same content".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        let msg2 = Model {
            id: 3,
            user_name: "charlie".to_string(),
            content: "Same content".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        let msg3 = Model {
            id: 4,
            user_name: "dave".to_string(),
            content: "Different content".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert_eq!(msg1, msg2);
        assert_ne!(msg1, msg3);
    }

    #[test]
    fn test_message_serialization() {
        let now = Utc::now();
        let msg = Model {
            id: 5,
            user_name: "user5".to_string(),
            content: "Serializable message".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        let json = serde_json::to_string(&msg);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("user5"));
        assert!(json_str.contains("Serializable message"));
    }

    #[test]
    fn test_message_deserialization() {
        let json_str = r#"{
            "id": 6,
            "user_name": "user6",
            "content": "Deserializable message",
            "timestamp": 1686566400000
        }"#;

        let result = serde_json::from_str::<Model>(json_str);
        assert!(result.is_ok());

        let msg = result.unwrap();
        assert_eq!(msg.id, 6);
        assert_eq!(msg.user_name, "user6");
        assert_eq!(msg.content, "Deserializable message");
    }

    #[test]
    fn test_message_with_special_characters() {
        let now = Utc::now();
        let msg = Model {
            id: 7,
            user_name: "user@123".to_string(),
            content: "Content with 中文, emoji 😊, and !@#$%^&*()".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert_eq!(msg.user_name, "user@123");
        assert!(msg.content.contains("中文"));
        assert!(msg.content.contains("😊"));
        assert!(msg.content.contains("!@#$%^&*()"));
    }

    #[test]
    fn test_message_with_multiline_content() {
        let now = Utc::now();
        let content = "Line 1\nLine 2\nLine 3\tTabbed";
        let msg = Model {
            id: 8,
            user_name: "user8".to_string(),
            content: content.to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert_eq!(msg.content, content);
        assert!(msg.content.contains('\n'));
        assert!(msg.content.contains('\t'));
    }

    #[test]
    fn test_message_empty_content() {
        let now = Utc::now();
        let msg = Model {
            id: 9,
            user_name: "user9".to_string(),
            content: String::new(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert!(msg.content.is_empty());
    }

    #[test]
    fn test_message_long_content() {
        let now = Utc::now();
        let long_content = "x".repeat(10000);
        let msg = Model {
            id: 10,
            user_name: "user10".to_string(),
            content: long_content.clone(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        assert_eq!(msg.content, long_content);
        assert_eq!(msg.content.len(), 10000);
    }

    #[test]
    fn test_message_round_trip_serialization() {
        let now = Utc::now();
        let original = Model {
            id: 11,
            user_name: "user11".to_string(),
            content: "Round trip test".to_string(),
            timestamp: now,
            room_id: None,
            user_id: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Model = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.user_name, deserialized.user_name);
        assert_eq!(original.content, deserialized.content);
        
        // timestamp is serialized as milliseconds, so compare with millisecond precision
        assert_eq!(
            original.timestamp.timestamp_millis(),
            deserialized.timestamp.timestamp_millis()
        );
    }
}