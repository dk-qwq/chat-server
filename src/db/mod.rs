pub mod messages;
pub mod users;

use crate::{entity::{message, user}, state::{MessageDb, UserDb}};
use sea_orm::{ConnectionTrait, Schema};

pub async fn init_user_table(db: &UserDb) {
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    db.execute(
        schema
            .create_table_from_entity(user::Entity)
            .if_not_exists(),
    )
    .await
    .unwrap();
}

pub async fn init_message_table(db: &MessageDb) {
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    db.execute(
        schema
            .create_table_from_entity(message::Entity)
            .if_not_exists(),
    )
    .await
    .unwrap();
}
