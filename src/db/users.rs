#![allow(dead_code)]

use std::ops::Deref;

use rand::{RngExt, distr::Alphabetic};
use sea_orm::{ActiveModelTrait, DbErr};
use crate::{entity::user, db::UserDb};

use sea_orm::Set;

fn gen_token(length: usize) -> String {
    let mut rng = rand::rng();

    (0..length)
        .map(|_| rng.sample(Alphabetic) as char)
        .collect()
}

pub async fn create_user(
    db: &UserDb,
    form: user::Model
) -> Result<user::Model, DbErr> {
    let active_model = user::ActiveModel {
        user_name: Set(form.user_name),
        password: Set(form.password),
        token: Set(gen_token(128)),
        ..Default::default()
    };
    active_model.insert(db.deref()).await
}

pub async fn find_by_user_name(
    db: &UserDb,
    user_name: String
) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find_by_user_name(user_name).one(db.deref()).await
}

pub async fn find_by_token(
    db: &UserDb,
    token: String
) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find_by_token(token).one(db.deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, ConnectionTrait};

    async fn setup_test_db() -> UserDb {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory SQLite");
        let user_db = UserDb(db);

        let builder = user_db.get_database_backend();
        let schema = sea_orm::Schema::new(builder);
        user_db
            .execute(
                schema
                    .create_table_from_entity(user::Entity)
                    .if_not_exists(),
            )
            .await
            .expect("Failed to initialize user table");

        user_db
    }

    #[test]
    fn test_gen_token_length() {
        let token = gen_token(32);
        assert_eq!(token.len(), 32);

        let token = gen_token(128);
        assert_eq!(token.len(), 128);

        let token = gen_token(0);
        assert_eq!(token.len(), 0);
    }

    #[test]
    fn test_gen_token_alphabetic_only() {
        let token = gen_token(100);
        assert!(token.chars().all(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn test_gen_token_unique() {
        let token1 = gen_token(50);
        let token2 = gen_token(50);
        // Token should be unique (very high probability)
        assert_ne!(token1, token2);
    }

    #[tokio::test]
    async fn test_create_user_basic() {
        let db = setup_test_db().await;
        let user = user::Model {
            id: 0,
            user_name: "alice".to_string(),
            password: "secret123".to_string(),
            token: String::new(),
        };

        let result = create_user(&db, user.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.user_name, "alice");
        assert_eq!(created.password, "secret123");
        assert_eq!(created.token.len(), 128); // Default token length
        assert!(created.token.chars().all(|c| c.is_ascii_alphabetic()));
        assert!(created.id > 0);
    }

    #[tokio::test]
    async fn test_create_user_special_characters() {
        let db = setup_test_db().await;
        let user = user::Model {
            id: 0,
            user_name: "user@123".to_string(),
            password: "pass!@#$%^&*()".to_string(),
            token: String::new(),
        };

        let result = create_user(&db, user.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.user_name, "user@123");
        assert_eq!(created.password, "pass!@#$%^&*()");
    }

    #[tokio::test]
    async fn test_create_multiple_users() {
        let db = setup_test_db().await;

        for i in 0..5 {
            let user = user::Model {
                id: 0,
                user_name: format!("user{}", i),
                password: format!("password{}", i),
                token: String::new(),
            };
            let result = create_user(&db, user).await;
            assert!(result.is_ok());
        }

        // Verify each user has unique token
        let user1_result = find_by_user_name(&db, "user0".to_string()).await;
        let user2_result = find_by_user_name(&db, "user1".to_string()).await;

        assert!(user1_result.is_ok());
        assert!(user2_result.is_ok());

        let user1 = user1_result.unwrap().unwrap();
        let user2 = user2_result.unwrap().unwrap();
        assert_ne!(user1.token, user2.token);
    }

    #[tokio::test]
    async fn test_find_by_user_name_exists() {
        let db = setup_test_db().await;
        let user = user::Model {
            id: 0,
            user_name: "bob".to_string(),
            password: "bobpass".to_string(),
            token: String::new(),
        };

        let created = create_user(&db, user).await.unwrap();

        let result = find_by_user_name(&db, "bob".to_string()).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_user = found.unwrap();
        assert_eq!(found_user.user_name, "bob");
        assert_eq!(found_user.password, "bobpass");
        assert_eq!(found_user.token, created.token);
    }

    #[tokio::test]
    async fn test_find_by_user_name_not_exists() {
        let db = setup_test_db().await;

        let result = find_by_user_name(&db, "nonexistent".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_find_by_user_name_case_sensitive() {
        let db = setup_test_db().await;
        let user = user::Model {
            id: 0,
            user_name: "TestUser".to_string(),
            password: "pass".to_string(),
            token: String::new(),
        };

        let _ = create_user(&db, user).await;

        // Try to find with different case
        let result = find_by_user_name(&db, "testuser".to_string()).await;

        assert!(result.is_ok());
        // User names are case-sensitive, so this should not be found
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_find_by_token_exists() {
        let db = setup_test_db().await;
        let user = user::Model {
            id: 0,
            user_name: "charlie".to_string(),
            password: "charliepass".to_string(),
            token: String::new(),
        };

        let created = create_user(&db, user).await.unwrap();
        let token = created.token.clone();

        let result = find_by_token(&db, token).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_user = found.unwrap();
        assert_eq!(found_user.user_name, "charlie");
    }

    #[tokio::test]
    async fn test_find_by_token_not_exists() {
        let db = setup_test_db().await;

        let result = find_by_token(&db, "invalid_token_12345".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_find_by_token_multiple_users() {
        let db = setup_test_db().await;

        // Create two users
        let user1 = user::Model {
            id: 0,
            user_name: "user1".to_string(),
            password: "pass1".to_string(),
            token: String::new(),
        };
        let user2 = user::Model {
            id: 0,
            user_name: "user2".to_string(),
            password: "pass2".to_string(),
            token: String::new(),
        };

        let created1 = create_user(&db, user1).await.unwrap();
        let created2 = create_user(&db, user2).await.unwrap();

        // Each token should find the correct user
        let found1 = find_by_token(&db, created1.token.clone())
            .await
            .unwrap()
            .unwrap();
        let found2 = find_by_token(&db, created2.token.clone())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found1.user_name, "user1");
        assert_eq!(found2.user_name, "user2");
    }

    #[tokio::test]
    async fn test_token_uniqueness_across_users() {
        let db = setup_test_db().await;

        let mut tokens = Vec::new();

        for i in 0..10 {
            let user = user::Model {
                id: 0,
                user_name: format!("user{}", i),
                password: format!("pass{}", i),
                token: String::new(),
            };
            let created = create_user(&db, user).await.unwrap();
            tokens.push(created.token);
        }

        // All tokens should be unique
        let unique_count = tokens.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 10);
    }
}