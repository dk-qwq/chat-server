#![allow(dead_code)]

use rand::{RngExt, distr::Alphabetic};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr};
use crate::entity::users;

use sea_orm::Set;

fn gen_token(length: usize) -> String {
    let mut rng = rand::rng();

    (0..length)
        .map(|_| rng.sample(Alphabetic) as char)
        .collect()
}

pub async fn create_user(
    db: &impl ConnectionTrait,
    form: users::Model
) -> Result<users::Model, DbErr> {
    let active_model = users::ActiveModel {
        user_id: Set(form.user_id),
        user_name: Set(form.user_name),
        password: Set(form.password),
        token: Set(gen_token(128)),
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