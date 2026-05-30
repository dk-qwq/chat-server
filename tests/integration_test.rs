use axum_test::TestServer;
use chat_server::build_test_app;
use serde_json::{json, Value};
use axum::http::StatusCode;

/// 辅助函数：构建测试服务器
async fn test_server() -> TestServer {
    let app = build_test_app().await;
    TestServer::new(app).unwrap()
}

/// 辅助函数：注册用户
async fn register_user(server: &TestServer, user_name: &str, password: &str) -> (StatusCode, Value) {
    let response = server
        .post("/api/register")
        .json(&json!({
            "user_name": user_name,
            "password": password,
        }))
        .await;

    let status = response.status_code();
    let body: Value = response.json();
    (status, body)
}

/// 辅助函数：登录
async fn login_user(server: &TestServer, user_name: &str, password: &str) -> (StatusCode, Value) {
    let response = server
        .post("/api/login")
        .json(&json!({
            "user_name": user_name,
            "password": password,
        }))
        .await;

    let status = response.status_code();
    let body: Value = response.json();
    (status, body)
}

// ==================== 健康检查测试 ====================

#[tokio::test]
async fn test_health_check() {
    let server = test_server().await;
    let response = server.get("/health").await;
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.text(), "OK");
}

// ==================== 注册测试 ====================

#[tokio::test]
async fn test_register_success() {
    let server = test_server().await;
    let (status, body) = register_user(&server, "alice", "password123").await;
    assert_eq!(status, 201);
    assert_eq!(body["message"], "注册成功");
}

#[tokio::test]
async fn test_register_empty_username() {
    let server = test_server().await;
    let (status, body) = register_user(&server, "", "password123").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "用户名或密码不能为空");
}

#[tokio::test]
async fn test_register_empty_password() {
    let server = test_server().await;
    let (status, body) = register_user(&server, "alice", "").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "用户名或密码不能为空");
}

#[tokio::test]
async fn test_register_duplicate_user() {
    let server = test_server().await;
    // 第一次注册
    let (status, _) = register_user(&server, "alice", "password123").await;
    assert_eq!(status, 201);

    // 重复注册
    let (status, body) = register_user(&server, "alice", "another_password").await;
    assert_eq!(status, 409);
    assert_eq!(body["message"], "该用户名已被使用");
}

// ==================== 登录测试 ====================

#[tokio::test]
async fn test_login_success() {
    let server = test_server().await;
    // 先注册
    let (status, _) = register_user(&server, "bob", "secret123").await;
    assert_eq!(status, 201);

    // 登录
    let (status, body) = login_user(&server, "bob", "secret123").await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "登录成功");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = test_server().await;
    // 先注册
    register_user(&server, "bob", "secret123").await;

    // 用错误密码登录
    let (status, body) = login_user(&server, "bob", "wrong_password").await;
    assert_eq!(status, 401);
    assert_eq!(body["message"], "用户不存在或密码错误");
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let server = test_server().await;
    let (status, body) = login_user(&server, "nonexistent", "password").await;
    assert_eq!(status, 401);
    assert_eq!(body["message"], "用户不存在或密码错误");
}

#[tokio::test]
async fn test_login_empty_username() {
    let server = test_server().await;
    let (status, body) = login_user(&server, "", "password").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "用户名或密码不能为空");
}

#[tokio::test]
async fn test_login_empty_password() {
    let server = test_server().await;
    let (status, body) = login_user(&server, "bob", "").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "用户名或密码不能为空");
}

// ==================== Cookie 测试 ====================

#[tokio::test]
async fn test_login_sets_token_cookie() {
    let server = test_server().await;
    register_user(&server, "charlie", "pass123").await;

    let response = server
        .post("/api/login")
        .json(&json!({
            "user_name": "charlie",
            "password": "pass123",
        }))
        .await;

    assert_eq!(response.status_code(), 200);

    // 验证 Set-Cookie 头中包含 token
    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();

    assert!(
        cookies.iter().any(|c| c.starts_with("token=")),
        "响应应包含 token cookie，实际 cookies: {:?}",
        cookies
    );
}
