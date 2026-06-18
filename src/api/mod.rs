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
        ws::handler_ws,
    },
    middleware::auth::auth_middleware,
    state::AppState,
};

mod login;
mod me;
mod messages;
mod register;
mod ws;

pub fn init_api_router(app_state: AppState) -> Router {
    let public_router = Router::new()
        .route("/login", post(handler_login))
        .route("/register", post(handler_register));

    let protected_router = Router::new()
        .route("/me", get(handler_me))
        .route("/ws", any(handler_ws))
        .route("/messages", get(get_message))
        .route("/messages/meta", get(latest_message_id))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    public_router.merge(protected_router).with_state(app_state)
}
