use chat_server::db::user::{create_user, find_by_user_id};
use chat_server::entity::users;
use sea_orm::{Database, Schema, ConnectionTrait};

#[tokio::test]
async fn create_and_find_user() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    db.execute(
        schema.create_table_from_entity(users::Entity).if_not_exists()
    ).await.unwrap();

    let form = users::Model {
        id: 0,
        user_id: "u1".to_string(),
        user_name: "Alice".to_string(),
        password: "secret".to_string(),
    };

    let res = create_user(&db, form.clone()).await.expect("create_user failed");
    assert_eq!(res.user_id, form.user_id);
    assert_eq!(res.user_name, form.user_name);

    let found = find_by_user_id(&db, form.user_id.clone()).await.expect("find failed");
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.user_name, form.user_name);
}

#[tokio::test]
async fn duplicate_user_id_fails() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    db.execute(
        schema.create_table_from_entity(users::Entity).if_not_exists()
    ).await.unwrap();

    let form = users::Model {
        id: 0,
        user_id: "dup".to_string(),
        user_name: "Bob".to_string(),
        password: "pw".to_string(),
    };

    let _ = create_user(&db, form.clone()).await.expect("first insert failed");
    let second = create_user(&db, form.clone()).await;
    assert!(second.is_err(), "expected unique constraint error on duplicate user_id");
}

#[tokio::test]
async fn find_none_for_missing() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    db.execute(
        schema.create_table_from_entity(users::Entity).if_not_exists()
    ).await.unwrap();

    let found = find_by_user_id(&db, "nope".to_string()).await.expect("find failed");
    assert!(found.is_none());
}
