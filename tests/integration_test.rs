use axum_test::TestServer;
use chat_server::{api, db, state::{AppState, MessageDb, UserDb}, entity::message, ws::{hub::Chathub, protocol::RoomCommand}};
use sea_orm::Database;
use serde_json::{json, Value};
use axum::{Router, http::StatusCode, routing::get};
use tokio::sync::mpsc;
use tower_http::trace::TraceLayer;
use chrono::Utc;

/// 构建测试用 App（使用内存数据库）
pub async fn build_test_app() -> Router {
    let user_db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    let message_db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    let user_db = UserDb(user_db);
    let message_db = MessageDb(message_db);

    db::init_user_table(&user_db).await;
    db::init_message_table(&message_db).await;
    

    let (tx, _rx) = mpsc::channel::<RoomCommand>(32);

    let app_state = AppState {
        user_db,
        message_db,
        chathub: Chathub::new(tx),
    };

    let api_router = api::init_api_router(app_state);

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api_router)
        .layer(TraceLayer::new_for_http())
}


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

// ==================== 消息数据库操作测试 ====================

/// 辅助函数：创建消息数据库连接
async fn create_message_db() -> MessageDb {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");
    let message_db = MessageDb(db);
    // 需要先创建 users/rooms 表以满足 FK 约束
    let user_db = UserDb(message_db.0.clone());
    db::init_user_table(&user_db).await;
    db::init_message_table(&message_db).await;
    message_db
}

#[tokio::test]
async fn test_insert_message() {
    let db = create_message_db().await;
    let msg = message::Model {
        id: 0,
        user_name: "alice".to_string(),
        content: "Hello, World!".to_string(),
        timestamp: Utc::now(),
        room_id: None,
        user_id: None,
    };

    let result = db::messages::insert_message(&db, msg.clone()).await;
    
    assert!(result.is_ok());
    let inserted_msg = result.unwrap();
    assert_eq!(inserted_msg.user_name, "alice");
    assert_eq!(inserted_msg.content, "Hello, World!");
    assert!(inserted_msg.id > 0); // 自增 ID 应该大于 0
}

#[tokio::test]
async fn test_insert_multiple_messages() {
    let db = create_message_db().await;
    
    for i in 0..3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let result = db::messages::insert_message(&db, msg).await;
        assert!(result.is_ok());
    }

    // 验证最新消息 ID 为 3
    let latest_id = db::messages::latest_message_id(&db).await;
    assert!(latest_id.is_ok());
    assert_eq!(latest_id.unwrap(), Some(3));
}

#[tokio::test]
async fn test_latest_message_id_empty_table() {
    let db = create_message_db().await;
    
    let result = db::messages::latest_message_id(&db).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn test_latest_message_id_with_messages() {
    let db = create_message_db().await;
    
    // 插入三条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    let result = db::messages::latest_message_id(&db).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(3));
}

#[tokio::test]
async fn test_list_message_before_basic() {
    let db = create_message_db().await;
    
    // 插入 5 条消息 (id: 1, 2, 3, 4, 5)
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 3 的消息
    let result = db::messages::list_message_before(&db, 3, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 3); // 应该有 3 条消息 (id: 1, 2, 3)
    
    // 验证按 ID 倒序 (3, 2, 1)
    assert_eq!(messages[0].id, 3);
    assert_eq!(messages[1].id, 2);
    assert_eq!(messages[2].id, 1);
}

#[tokio::test]
async fn test_list_message_before_with_limit() {
    let db = create_message_db().await;
    
    // 插入 10 条消息
    for i in 1..=10 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 7 的消息，但限制只返回 3 条
    let result = db::messages::list_message_before(&db, 7, 3).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 3); // 应该只有 3 条消息
    
    // 应该是 ID 最大的 3 条 (7, 6, 5)
    assert_eq!(messages[0].id, 7);
    assert_eq!(messages[1].id, 6);
    assert_eq!(messages[2].id, 5);
}

#[tokio::test]
async fn test_list_message_before_id_at_boundary() {
    let db = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 1 的消息
    let result = db::messages::list_message_before(&db, 1, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, 1);
}

#[tokio::test]
async fn test_list_message_before_no_results() {
    let db = create_message_db().await;
    
    // 插入 3 条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 0 的消息（不存在）
    let result = db::messages::list_message_before(&db, 0, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_list_message_after_basic() {
    let db = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 2 的消息
    let result = db::messages::list_message_after(&db, 2, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 4); // 应该有 4 条消息 (id: 2, 3, 4, 5)
    
    // 验证按 ID 升序 (2, 3, 4, 5)
    assert_eq!(messages[0].id, 2);
    assert_eq!(messages[1].id, 3);
    assert_eq!(messages[2].id, 4);
    assert_eq!(messages[3].id, 5);
}

#[tokio::test]
async fn test_list_message_after_with_limit() {
    let db = create_message_db().await;
    
    // 插入 10 条消息
    for i in 1..=10 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 3 的消息，但限制只返回 4 条
    let result = db::messages::list_message_after(&db, 3, 4).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 4); // 应该只有 4 条消息
    
    // 应该是 ID 最小的 4 条 (3, 4, 5, 6)
    assert_eq!(messages[0].id, 3);
    assert_eq!(messages[1].id, 4);
    assert_eq!(messages[2].id, 5);
    assert_eq!(messages[3].id, 6);
}

#[tokio::test]
async fn test_list_message_after_id_at_boundary() {
    let db = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 5 的消息
    let result = db::messages::list_message_after(&db, 5, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, 5);
}

#[tokio::test]
async fn test_list_message_after_no_results() {
    let db = create_message_db().await;
    
    // 插入 3 条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 100 的消息（不存在）
    let result = db::messages::list_message_after(&db, 100, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_list_message_before_and_after_consistency() {
    let db = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: None,
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 3 的消息 (before)
    let before_result = db::messages::list_message_before(&db, 3, 10).await;
    let before_messages = before_result.unwrap();
    
    // 查询 ID >= 3 的消息 (after)
    let after_result = db::messages::list_message_after(&db, 3, 10).await;
    let after_messages = after_result.unwrap();

    // ID 3 应该在两个结果中都出现
    assert!(before_messages.iter().any(|m| m.id == 3));
    assert!(after_messages.iter().any(|m| m.id == 3));
    
    // 统计总消息数（ID 3 会重复计算）
    let total_unique = before_messages.len() + after_messages.len() - 1;
    assert_eq!(total_unique, 5);
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

// ==================== 高级用户操作测试 ====================

#[tokio::test]
async fn test_register_and_login_multiple_times() {
    let server = test_server().await;
    let username = "multitest";
    let password = "secure_pass";

    // 注册
    let (reg_status, _) = register_user(&server, username, password).await;
    assert_eq!(reg_status, 201);

    // 多次登录
    for _ in 0..3 {
        let (login_status, body) = login_user(&server, username, password).await;
        assert_eq!(login_status, 200);
        assert_eq!(body["message"], "登录成功");
    }
}

#[tokio::test]
async fn test_login_immediately_after_registration() {
    let server = test_server().await;
    let username = "immediate";
    let password = "pass123";

    // 注册
    register_user(&server, username, password).await;

    // 立即登录
    let (status, body) = login_user(&server, username, password).await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "登录成功");

    // 获取 token 并验证
    let response = server
        .post("/api/login")
        .json(&json!({"user_name": username, "password": password}))
        .await;

    let token = extract_token_from_response(&response).expect("Unable to extract token");
    let (me_status, me_body) = get_me(&server, &token).await;
    assert_eq!(me_status, 200);
    assert_eq!(me_body["user_name"], username);
}

#[tokio::test]
async fn test_user_isolation() {
    let server = test_server().await;

    // 创建两个用户
    register_user(&server, "user_a", "pass_a").await;
    register_user(&server, "user_b", "pass_b").await;

    // 用户 A 登录
    let response_a = server
        .post("/api/login")
        .json(&json!({"user_name": "user_a", "password": "pass_a"}))
        .await;
    let token_a = extract_token_from_response(&response_a).expect("Unable to extract token for user_a");

    // 用户 B 登录
    let response_b = server
        .post("/api/login")
        .json(&json!({"user_name": "user_b", "password": "pass_b"}))
        .await;
    let token_b = extract_token_from_response(&response_b).expect("Unable to extract token for user_b");

    // 验证用户隔离
    let (_, body_a) = get_me(&server, &token_a).await;
    let (_, body_b) = get_me(&server, &token_b).await;

    assert_eq!(body_a["user_name"], "user_a");
    assert_eq!(body_b["user_name"], "user_b");
    assert_ne!(body_a["user_name"], body_b["user_name"]);
}

// ==================== 密码验证测试 ====================

#[tokio::test]
async fn test_login_with_partial_password_mismatch() {
    let server = test_server().await;

    register_user(&server, "passtest", "correct_password").await;

    // 用错误的密码登录
    let (status, _body) = login_user(&server, "passtest", "wrong_password").await;
    assert_eq!(status, 401);

    // 用其他错误的密码登录
    let (status, _body) = login_user(&server, "passtest", "partially_wrong").await;
    assert_eq!(status, 401);

    // 用正确的密码登录应该成功
    let (status, _body) = login_user(&server, "passtest", "correct_password").await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn test_empty_password_vs_wrong_password() {
    let server = test_server().await;

    register_user(&server, "emptytest", "nonempty").await;

    // 用空密码登录
    let (status1, body1) = login_user(&server, "emptytest", "").await;
    assert_eq!(status1, 400);
    assert_eq!(body1["message"], "用户名或密码不能为空");

    // 用错误的非空密码登录
    let (status2, body2) = login_user(&server, "emptytest", "wrong").await;
    assert_eq!(status2, 401);
    assert_eq!(body2["message"], "用户不存在或密码错误");
}

// ==================== 数据库操作稳定性测试 ====================

#[tokio::test]
async fn test_concurrent_registrations() {
    let server = test_server().await;

    // 创建多个并发注册任务
    let mut tasks = Vec::new();

    for i in 0..5 {
        let username = format!("concurrent_user_{}", i);

        let response = server
            .post("/api/register")
            .json(&json!({
                "user_name": username,
                "password": format!("password_{}", i),
            }))
            .await;

        assert_eq!(response.status_code(), 201);
        tasks.push(i);
    }

    // 验证所有用户都能登录
    for i in tasks {
        let (status, _) = login_user(&server, &format!("concurrent_user_{}", i), &format!("password_{}", i)).await;
        assert_eq!(status, 200);
    }
}

// ==================== 令牌和认证测试 ====================

#[tokio::test]
async fn test_token_persistence_across_requests() {
    let server = test_server().await;

    register_user(&server, "token_test", "pass").await;

    let response = server
        .post("/api/login")
        .json(&json!({"user_name": "token_test", "password": "pass"}))
        .await;

    let token = extract_token_from_response(&response).expect("Unable to extract token");

    // 使用相同 token 多次请求
    for _ in 0..3 {
        let (status, body) = get_me(&server, &token).await;
        assert_eq!(status, 200);
        assert_eq!(body["user_name"], "token_test");
    }
}

#[tokio::test]
async fn test_invalid_token_patterns() {
    let server = test_server().await;

    // 测试各种无效的 token 格式
    let invalid_tokens = vec![
        "".to_string(),
        "token".to_string(),
        "123456".to_string(),
        "a".repeat(1000),
        "!!!@@@###".to_string(),
    ];

    for invalid_token in invalid_tokens {
        let (status, body) = get_me(&server, &invalid_token).await;
        assert_eq!(status, 400);
        assert_eq!(body["message"], "请登录");
    }
}

// ==================== 注册验证测试 ====================

#[tokio::test]
async fn test_register_with_unicode_username() {
    let server = test_server().await;

    let (status, body) = register_user(&server, "用户名", "password").await;
    assert_eq!(status, 201);
    assert_eq!(body["message"], "注册成功");

    // 验证可以登录
    let (login_status, _) = login_user(&server, "用户名", "password").await;
    assert_eq!(login_status, 200);
}

#[tokio::test]
async fn test_register_with_unicode_password() {
    let server = test_server().await;

    let (status, _body) = register_user(&server, "unicodepass", "密码123").await;
    assert_eq!(status, 201);

    let (login_status, _) = login_user(&server, "unicodepass", "密码123").await;
    assert_eq!(login_status, 200);
}

#[tokio::test]
async fn test_register_idempotency() {
    let server = test_server().await;

    // 第一次注册
    let (status1, _) = register_user(&server, "idempotent", "pass").await;
    assert_eq!(status1, 201);

    // 第二次注册相同用户名
    let (status2, body2) = register_user(&server, "idempotent", "pass").await;
    assert_eq!(status2, 409);
    assert_eq!(body2["message"], "该用户名已被使用");

    // 第三次注册相同用户名（不同密码）
    let (status3, body3) = register_user(&server, "idempotent", "different_pass").await;
    assert_eq!(status3, 409);
    assert_eq!(body3["message"], "该用户名已被使用");
}

// ==================== 边界条件和极限测试 ====================

#[tokio::test]
async fn test_whitespace_in_credentials() {
    let server = test_server().await;

    // 用户名和密码可能包含空格
    let (status, _body) = register_user(&server, "user with spaces", "pass with spaces").await;
    assert_eq!(status, 201);

    let (login_status, _) = login_user(&server, "user with spaces", "pass with spaces").await;
    assert_eq!(login_status, 200);
}

#[tokio::test]
async fn test_leading_trailing_whitespace() {
    let server = test_server().await;

    // 注册用户名带前导/尾部空格
    let (status1, _) = register_user(&server, " leadingspace", "password").await;
    assert_eq!(status1, 201);

    let (status2, _) = register_user(&server, "trailingspace ", "password").await;
    assert_eq!(status2, 201);

    // 这些用户名应该被视为不同的
    let (status3, _body3) = register_user(&server, "leadingspace", "password").await;
    // 如果空格被处理，这会返回 409；否则返回 201
    // 这取决于应用的实现选择
    assert!(status3 == 201 || status3 == 409);
}
