use std::ops::Deref;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

use crate::ws::hub::Chathub;

#[derive(Clone)]
pub struct UserDb(pub DatabaseConnection);

impl Deref for UserDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct MessageDb(pub DatabaseConnection);

impl Deref for MessageDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub user_db: UserDb,
    pub message_db: MessageDb,
    pub chathub: Chathub,
}