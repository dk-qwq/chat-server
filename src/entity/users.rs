#![allow(dead_code)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(unique, auto_increment = false)]
    pub user_id: String,
    pub user_name: String,
    pub password: String,
    pub token: String,
}


impl ActiveModelBehavior for ActiveModel {}