use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub fn json_resp(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_json_resp_ok() {
        let resp = json_resp(StatusCode::OK, "success");
        assert_eq!(resp.status(), StatusCode::OK);

        let (_parts, body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["message"], "success");
    }

    #[tokio::test]
    async fn test_json_resp_created() {
        let resp = json_resp(StatusCode::CREATED, "注册成功");
        assert_eq!(resp.status(), StatusCode::CREATED);

        let (_parts, body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["message"], "注册成功");
    }

    #[tokio::test]
    async fn test_json_resp_bad_request() {
        let resp = json_resp(StatusCode::BAD_REQUEST, "用户名或密码不能为空");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let (_parts, body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["message"], "用户名或密码不能为空");
    }

    #[tokio::test]
    async fn test_json_resp_conflict() {
        let resp = json_resp(StatusCode::CONFLICT, "房间名重复");
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_json_resp_internal_server_error() {
        let resp = json_resp(StatusCode::INTERNAL_SERVER_ERROR, "server error");
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_json_resp_content_type() {
        let resp = json_resp(StatusCode::OK, "test");
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());
        assert!(content_type.is_some());
        assert!(content_type.unwrap().contains("application/json"));
    }

    #[tokio::test]
    async fn test_json_resp_empty_message() {
        let resp = json_resp(StatusCode::OK, "");
        let (_parts, body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["message"], "");
    }

    #[tokio::test]
    async fn test_json_resp_unicode_message() {
        let resp = json_resp(StatusCode::OK, "你好，世界！");
        let (_parts, body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["message"], "你好，世界！");
    }
}
