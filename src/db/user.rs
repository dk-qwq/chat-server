#![allow(dead_code)]

use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbErr};
pub struct UserManager {
    db: DatabaseConnection,
}

use crate::entity::users;

use sea_orm::Set;

pub async fn create_user(
    db: &impl ConnectionTrait,
    form: users::Model
) -> Result<users::Model, DbErr> {
    let active_model = users::ActiveModel {
        user_id: Set(form.user_id),
        user_name: Set(form.user_name),
        password: Set(form.password),
        ..Default::default()
    };
    active_model.insert(db).await
}

pub async fn find_by_user_id(
    db: &impl ConnectionTrait,
    user_id: String
) -> Result<Option<users::Model>, DbErr> {
    users::Entity::find_by_user_id(user_id).one(db).await
}