pub mod users;

use crate::entity::user;
use sea_orm::{ConnectionTrait, Schema};

pub async fn init_user_table(db: &impl ConnectionTrait) {
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
