pub mod messages;
pub mod room_users;
pub mod rooms;
pub mod users;

use std::ops::Deref;

use crate::entity::{message, room, room_user, user};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Schema};

async fn connect(url: &str) -> DatabaseConnection {
    Database::connect(url).await.unwrap()
}

#[derive(Clone)]
pub struct UserDb(pub DatabaseConnection);

impl UserDb {
    pub async fn connect(url: &str) -> Self {
        let db = Self(connect(url).await);
        db.init_user_table().await;
        db
    }

    async fn init_user_table(&self) {
        let builder = self.get_database_backend();
        let schema = Schema::new(builder);

        self.execute(
            schema
                .create_table_from_entity(user::Entity)
                .if_not_exists(),
        )
        .await
        .unwrap();
    }
}

impl Deref for UserDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct MessageDb(pub DatabaseConnection);

impl MessageDb {
    pub async fn connect(url: &str) -> Self {
        let db = Self(connect(url).await);
        db.init_message_table().await;
        db
    }

    async fn init_message_table(&self) {
        let builder = self.get_database_backend();
        let schema = Schema::new(builder);

        self.execute(
            schema
                .create_table_from_entity(message::Entity)
                .if_not_exists(),
        )
        .await
        .unwrap();
    }
}

impl Deref for MessageDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct RoomDb(pub DatabaseConnection);

impl RoomDb {
    pub async fn connect(url: &str) -> Self {
        let db = Self(connect(url).await);
        db.init_room_table().await;
        db
    }

    async fn init_room_table(&self) {
        let builder = self.get_database_backend();
        let schema = Schema::new(builder);

        self.execute(
            schema
                .create_table_from_entity(room::Entity)
                .if_not_exists(),
        )
        .await
        .unwrap();
    }
}

impl Deref for RoomDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct RoomUserDb(pub DatabaseConnection);

impl RoomUserDb {
    pub async fn connect(url: &str) -> Self {
        let db = Self(connect(url).await);
        db.init_room_user_table().await;
        db
    }

    async fn init_room_user_table(&self) {
        let builder = self.get_database_backend();
        let schema = Schema::new(builder);

        self.execute(
            schema
                .create_table_from_entity(room_user::Entity)
                .if_not_exists(),
        )
        .await
        .unwrap();
    }
}

impl Deref for RoomUserDb {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
