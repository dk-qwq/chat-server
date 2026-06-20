use std::{collections::HashMap, ops::Deref, sync::Arc};

use sea_orm::{DbErr, EntityTrait};
use tokio::sync::{RwLock, mpsc};

use crate::{
    config,
    db::rooms,
    db::{MessageDb, RoomDb},
    entity::{RoomId, room},
    ws::{protocol::RoomCommand, room::RoomActor},
};

#[derive(Clone)]
pub struct Chathub {
    room_db: RoomDb,
    message_db: MessageDb,
    pub rooms: Arc<RwLock<HashMap<RoomId, mpsc::Sender<RoomCommand>>>>,
}

pub enum CreateRoomError {
    Db(DbErr),
    DuplicateRoomName,
}

impl Chathub {
    pub async fn new(room_db: RoomDb, message_db: MessageDb) -> Self {
        let rooms = room::Entity::find()
            .all(room_db.deref())
            .await
            .unwrap_or_else(|err| panic!("room db couldn't open, err: {err}"));

        let mut room_map = HashMap::with_capacity(rooms.len());

        for room in rooms {
            let room_actor = RoomActor::new(
                room.id.clone(),
                config::WS_CHANNEL_BUFFER,
                message_db.clone(),
            );

            room_map.insert(room.id, room_actor.sender());
            tokio::spawn(room_actor.run());
        }

        Chathub {
            room_db,
            message_db,
            rooms: Arc::new(RwLock::new(room_map)),
        }
    }

    pub async fn create_room(&self, form: room::Model) -> Result<RoomId, CreateRoomError> {
        let room_name = form.room_name.clone();

        let existing = rooms::find_by_room_name(&self.room_db, room_name)
            .await
            .map_err(CreateRoomError::Db)?;

        if existing.is_some() {
            return Err(CreateRoomError::DuplicateRoomName);
        }

        let room = rooms::create_room(&self.room_db, form)
            .await
            .map_err(CreateRoomError::Db)?;

        let room_actor = RoomActor::new(
            room.id.clone(),
            config::WS_CHANNEL_BUFFER,
            self.message_db.clone(),
        );

        let mut rooms = self.rooms.write().await;
        rooms.insert(room.id.clone(), room_actor.sender());

        tokio::spawn(room_actor.run());
        Ok(room.id)
    }

    pub async fn get_room_sender(self, room_id: RoomId) -> Option<mpsc::Sender<RoomCommand>> {
        let res = self.rooms.read().await;

        (*res).get(&room_id).cloned()
    }
}
