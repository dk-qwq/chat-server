use axum::{
    Router, middleware,
    routing::{any, get, post},
};

use crate::{
    api::{
        login::handler_login,
        me::handler_me,
        messages::{get_message, latest_message_id},
        register::handler_register,
        rooms::handler_create_room,
        ws::handler_ws,
    },
    middleware::{auth::auth_middleware, room_auth},
    state::AppState,
};

mod login;
mod me;
mod messages;
mod register;
mod rooms;
mod ws;

pub fn init_api_router(app_state: AppState) -> Router {
    let public_router = Router::new()
        .route("/login", post(handler_login))
        .route("/register", post(handler_register));

    let auth_router = Router::new()
        .route("/me", get(handler_me))
        .route("/rooms", post(handler_create_room))
        // .route("/rooms", get(handler_get_rooms))
        ;

    let room_router = Router::new()
        .route("/rooms/{room_id}/ws", any(handler_ws))
        .route("/rooms/{room_id}/messages", get(get_message))
        .route("/rooms/{room_id}/messages/meta", get(latest_message_id))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            room_auth::locate_room,
        ));

    let protect_router =
        auth_router
            .merge(room_router)
            .route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            ));

    public_router.merge(protect_router).with_state(app_state)
}
