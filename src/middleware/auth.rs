use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use serde_json::json;

use crate::{db::users, db::UserDb};

pub async fn auth_middleware(
    cookie_jar: CookieJar,
    State(db): State<UserDb>,
    mut request: Request,
    next: Next,
) -> Response {
    let db_error = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "message": "远程连接错误",
        })),
    )
        .into_response();

    let no_user = (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({
            "message": "请登录",
        })),
    )
        .into_response();

    let Some(token) = cookie_jar.get("token") else {
        return no_user;
    };

    match users::find_by_token(&db, token.value().to_string()).await {
        Ok(Some(user)) => {
            request.extensions_mut().insert(user);

            next.run(request).await
        }
        Ok(None) => no_user,
        Err(_) => db_error,
    }
}
