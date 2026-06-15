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

/// 辅助函数：登录并返回 token
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

/// 辅助函数：从响应中提取 token
fn extract_token_from_response(response: &axum_test::TestResponse) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .find_map(|v| {
            let cookie_str = v.to_str().ok()?;
            if cookie_str.starts_with("token=") {
                let token = cookie_str
                    .split(';')
                    .next()?
                    .strip_prefix("token=")?
                    .to_string();
                Some(token)
            } else {
                None
            }
        })
}

/// 辅助函数：获取用户信息（需要有效的 token）
async fn get_me(server: &TestServer, token: &str) -> (StatusCode, Value) {
    let response = server
        .get("/api/me")
        .add_header("cookie", &format!("token={}", token))
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

// ==================== /api/me 测试 ====================

#[tokio::test]
async fn test_me_without_token() {
    let server = test_server().await;
    let (status, body) = get_me(&server, "").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "请登录");
}

#[tokio::test]
async fn test_me_with_invalid_token() {
    let server = test_server().await;
    let (status, body) = get_me(&server, "invalid_token_12345").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "请登录");
}

#[tokio::test]
async fn test_me_with_valid_token() {
    let server = test_server().await;
    // 注册用户
    register_user(&server, "dave", "password123").await;

    // 登录获取 token
    let login_response = server
        .post("/api/login")
        .json(&json!({
            "user_name": "dave",
            "password": "password123",
        }))
        .await;

    assert_eq!(login_response.status_code(), 200);

    // 从响应头中提取 token
    let token = extract_token_from_response(&login_response)
        .expect("Unable to extract token from login response");

    // 使用 token 访问 /api/me
    let (status, body) = get_me(&server, &token).await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "验证成功");
    assert_eq!(body["user_name"], "dave");
}

#[tokio::test]
async fn test_me_multiple_users() {
    let server = test_server().await;

    // 注册两个用户
    register_user(&server, "user1", "pass1").await;
    register_user(&server, "user2", "pass2").await;

    // 用户1登录
    let response1 = server
        .post("/api/login")
        .json(&json!({
            "user_name": "user1",
            "password": "pass1",
        }))
        .await;
    let token1 = extract_token_from_response(&response1).expect("Unable to extract token for user1");

    // 用户2登录
    let response2 = server
        .post("/api/login")
        .json(&json!({
            "user_name": "user2",
            "password": "pass2",
        }))
        .await;
    let token2 = extract_token_from_response(&response2).expect("Unable to extract token for user2");

    // 验证用户1的 token 返回用户1的信息
    let (status1, body1) = get_me(&server, &token1).await;
    assert_eq!(status1, 200);
    assert_eq!(body1["user_name"], "user1");

    // 验证用户2的 token 返回用户2的信息
    let (status2, body2) = get_me(&server, &token2).await;
    assert_eq!(status2, 200);
    assert_eq!(body2["user_name"], "user2");
}

// ==================== 注册后立即登录测试 ====================

#[tokio::test]
async fn test_register_then_immediate_login() {
    let server = test_server().await;
    
    // 注册用户
    let (status, _) = register_user(&server, "newuser", "securepass").await;
    assert_eq!(status, 201);

    // 立即登录（不间隔其他操作）
    let (login_status, login_body) = login_user(&server, "newuser", "securepass").await;
    assert_eq!(login_status, 200);
    assert_eq!(login_body["message"], "登录成功");
}

// ==================== 登录后注册相同用户测试 ====================

#[tokio::test]
async fn test_login_after_register_same_user() {
    let server = test_server().await;
    
    // 第一次注册
    let (status1, _) = register_user(&server, "duplicate", "password").await;
    assert_eq!(status1, 201);

    // 第一次登录
    let (status2, body2) = login_user(&server, "duplicate", "password").await;
    assert_eq!(status2, 200);
    assert_eq!(body2["message"], "登录成功");

    // 第二次注册相同用户
    let (status3, body3) = register_user(&server, "duplicate", "password").await;
    assert_eq!(status3, 409);
    assert_eq!(body3["message"], "该用户名已被使用");

    // 再次登录相同用户（确保仍然有效）
    let (status4, body4) = login_user(&server, "duplicate", "password").await;
    assert_eq!(status4, 200);
    assert_eq!(body4["message"], "登录成功");
}

// ==================== 边界条件测试 ====================

#[tokio::test]
async fn test_register_with_special_characters() {
    let server = test_server().await;
    let (status, body) = register_user(&server, "user@123", "pass!@#$%").await;
    assert_eq!(status, 201);
    assert_eq!(body["message"], "注册成功");

    // 验证可以用相同凭证登录
    let (login_status, login_body) = login_user(&server, "user@123", "pass!@#$%").await;
    assert_eq!(login_status, 200);
    assert_eq!(login_body["message"], "登录成功");
}

#[tokio::test]
async fn test_login_case_sensitive_username() {
    let server = test_server().await;
    
    // 注册用户
    register_user(&server, "TestUser", "password").await;

    // 用不同大小写尝试登录
    let (status, _body) = login_user(&server, "testuser", "password").await;
    // 用户名通常应该是大小写敏感的，所以这应该返回错误
    // 但这取决于实现
    // 提示用户检查是否应该支持大小写不敏感
    // 我们这里就假设用户名是大小写敏感的
    assert_eq!(status, 401);
}

#[tokio::test]
async fn test_register_long_username() {
    let server = test_server().await;
    let long_username = "a".repeat(255);
    let (status, _body) = register_user(&server, &long_username, "password").await;
    // 长用户名应该能够注册（取决于数据库限制）
    // 如果返回 201，表示成功；如果返回错误，也是可以接受的
    // 这里我们期望成功
    assert!(status == 201 || status == 400);
}

// ==================== 会话相关测试 ====================

#[tokio::test]
async fn test_multiple_login_sessions() {
    let server = test_server().await;
    
    // 注册用户
    register_user(&server, "sessionuser", "password").await;

    // 第一次登录
    let response1 = server
        .post("/api/login")
        .json(&json!({
            "user_name": "sessionuser",
            "password": "password",
        }))
        .await;
    let token1 = extract_token_from_response(&response1).expect("Unable to extract token");

    // 第二次登录
    let response2 = server
        .post("/api/login")
        .json(&json!({
            "user_name": "sessionuser",
            "password": "password",
        }))
        .await;
    let token2 = extract_token_from_response(&response2).expect("Unable to extract token");

    // 两个 token 可能不同（取决于实现是否生成新 token 或返回相同 token）
    // 这里我们验证两个 token 都有效
    let (status1, _) = get_me(&server, &token1).await;
    let (status2, _) = get_me(&server, &token2).await;
    
    assert_eq!(status1, 200);
    assert_eq!(status2, 200);
}
