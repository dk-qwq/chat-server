use axum::extract::FromRef;

use crate::{
    db::{MessageDb, RoomDb, RoomUserDb, UserDb},
    ws::hub::Chathub,
};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub user_db: UserDb,
    pub room_db: RoomDb,
    pub message_db: MessageDb,
    pub room_users_db: RoomUserDb,
    pub chathub: Chathub,
}
