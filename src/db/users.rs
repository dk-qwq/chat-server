#![allow(dead_code)]

use rand::{RngExt, distr::Alphabetic};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr};
use crate::{entity::user};

use sea_orm::Set;

fn gen_token(length: usize) -> String {
    let mut rng = rand::rng();

    (0..length)
        .map(|_| rng.sample(Alphabetic) as char)
        .collect()
}

pub async fn create_user(
    db: &impl ConnectionTrait,
    form: user::Model
) -> Result<user::Model, DbErr> {
    let active_model = user::ActiveModel {
        user_name: Set(form.user_name),
        password: Set(form.password),
        token: Set(gen_token(128)),
        ..Default::default()
    };
    active_model.insert(db).await
}

pub async fn find_by_user_name(
    db: &impl ConnectionTrait,
    user_name: String
) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find_by_user_name(user_name).one(db).await
}

pub async fn find_by_token(
    db: &impl ConnectionTrait,
    token: String
) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find_by_token(token).one(db).await
}