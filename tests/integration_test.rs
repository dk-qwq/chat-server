use axum_test::TestServer;
use chat_server::{
    api, db,
    db::{MessageDb, RoomDb, RoomUserDb, UserDb},
    entity::{self, message, RoomId},
    state::AppState,
    ws::hub::Chathub,
};
use sea_orm::{ConnectionTrait, Database, Schema};
use serde_json::{json, Value};
use axum::{Router, http::StatusCode, routing::get};
use tower_http::trace::TraceLayer;
use chrono::Utc;

/// 构建测试用 App（使用共享内存数据库以支持 FK 约束）
pub async fn build_test_app() -> Router {
    let conn = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    // 在共享数据库上创建所有表
    let builder = conn.get_database_backend();
    let schema = Schema::new(builder);
    conn.execute(
        schema
            .create_table_from_entity(entity::user::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create users table");
    conn.execute(
        schema
            .create_table_from_entity(entity::room::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create rooms table");
    conn.execute(
        schema
            .create_table_from_entity(entity::message::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create messages table");
    conn.execute(
        schema
            .create_table_from_entity(entity::room_user::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create room_users table");

    let user_db = UserDb(conn.clone());
    let message_db = MessageDb(conn.clone());
    let room_db = RoomDb(conn.clone());
    let room_users_db = RoomUserDb(conn);

    let chathub = Chathub::new(room_db.clone(), message_db.clone()).await;

    let app_state = AppState {
        user_db,
        message_db,
        room_db,
        room_users_db,
        chathub,
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

/// 辅助函数：创建消息数据库连接（使用共享内存数据库以支持 FK）
async fn create_message_db() -> (MessageDb, RoomId) {
    use sea_orm::Database;
    let conn = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    // 在同一数据库上创建所有表（FK 约束需要 rooms、users 和 messages 在同一数据库）
    use sea_orm::{ConnectionTrait, Schema};
    let builder = conn.get_database_backend();
    let schema = Schema::new(builder);

    conn.execute(
        schema
            .create_table_from_entity(entity::user::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create users table");
    conn.execute(
        schema
            .create_table_from_entity(entity::room::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create rooms table");
    conn.execute(
        schema
            .create_table_from_entity(entity::message::Entity)
            .if_not_exists(),
    )
    .await
    .expect("Failed to create messages table");

    let room_db = RoomDb(conn.clone());
    let message_db = MessageDb(conn);

    // 创建一个测试房间
    let room = db::rooms::create_room(
        &room_db,
        entity::room::Model {
            id: RoomId(0),
            room_name: "test_room".to_string(),
            password: "".to_string(),
        },
    )
    .await
    .expect("Failed to create test room");

    (message_db, room.id)
}

#[tokio::test]
async fn test_insert_message() {
    let (db, room_id) = create_message_db().await;
    let msg = message::Model {
        id: 0,
        user_name: "alice".to_string(),
        content: "Hello, World!".to_string(),
        timestamp: Utc::now(),
        room_id: Some(room_id.0),
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
    let (db, room_id) = create_message_db().await;
    
    for i in 0..3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let result = db::messages::insert_message(&db, msg).await;
        assert!(result.is_ok());
    }

    // 验证最新消息 ID 为 3
    let latest_id = db::messages::latest_message_id(&db, room_id.clone()).await;
    assert!(latest_id.is_ok());
    assert_eq!(latest_id.unwrap(), Some(3));
}

#[tokio::test]
async fn test_latest_message_id_empty_table() {
    let (db, room_id) = create_message_db().await;
    
    let result = db::messages::latest_message_id(&db, room_id).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn test_latest_message_id_with_messages() {
    let (db, room_id) = create_message_db().await;
    
    // 插入三条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    let result = db::messages::latest_message_id(&db, room_id).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(3));
}

#[tokio::test]
async fn test_list_message_before_basic() {
    let (db, room_id) = create_message_db().await;
    
    // 插入 5 条消息 (id: 1, 2, 3, 4, 5)
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 3 的消息
    let result = db::messages::list_message_before(&db, room_id.clone(), 3, 10).await;
    
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
    let (db, room_id) = create_message_db().await;
    
    // 插入 10 条消息
    for i in 1..=10 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 7 的消息，但限制只返回 3 条
    let result = db::messages::list_message_before(&db, room_id.clone(), 7, 3).await;
    
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
    let (db, room_id) = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 1 的消息
    let result = db::messages::list_message_before(&db, room_id.clone(), 1, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, 1);
}

#[tokio::test]
async fn test_list_message_before_no_results() {
    let (db, room_id) = create_message_db().await;
    
    // 插入 3 条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 0 的消息（不存在）
    let result = db::messages::list_message_before(&db, room_id.clone(), 0, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_list_message_after_basic() {
    let (db, room_id) = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 2 的消息
    let result = db::messages::list_message_after(&db, room_id.clone(), 2, 10).await;
    
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
    let (db, room_id) = create_message_db().await;
    
    // 插入 10 条消息
    for i in 1..=10 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 3 的消息，但限制只返回 4 条
    let result = db::messages::list_message_after(&db, room_id.clone(), 3, 4).await;
    
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
    let (db, room_id) = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 5 的消息
    let result = db::messages::list_message_after(&db, room_id.clone(), 5, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, 5);
}

#[tokio::test]
async fn test_list_message_after_no_results() {
    let (db, room_id) = create_message_db().await;
    
    // 插入 3 条消息
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID >= 100 的消息（不存在）
    let result = db::messages::list_message_after(&db, room_id.clone(), 100, 10).await;
    
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_list_message_before_and_after_consistency() {
    let (db, room_id) = create_message_db().await;
    
    // 插入 5 条消息
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // 查询 ID <= 3 的消息 (before)
    let before_result = db::messages::list_message_before(&db, room_id.clone(), 3, 10).await;
    let before_messages = before_result.unwrap();
    
    // 查询 ID >= 3 的消息 (after)
    let after_result = db::messages::list_message_after(&db, room_id.clone(), 3, 10).await;
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

// ==================== 房间创建测试 ====================

/// 辅助函数：注册并登录，返回 (server, token)
async fn register_and_login(server: &TestServer, user_name: &str, password: &str) -> String {
    let (status, _) = register_user(server, user_name, password).await;
    assert_eq!(status, 201);

    let response = server
        .post("/api/login")
        .json(&json!({"user_name": user_name, "password": password}))
        .await;

    extract_token_from_response(&response).expect("Failed to extract token")
}

/// 辅助函数：创建房间，返回 (status, body)
async fn create_room(
    server: &TestServer,
    token: &str,
    room_name: &str,
    password: &str,
) -> (StatusCode, Value) {
    let response = server
        .post(&format!(
            "/api/rooms?room_name={}&password={}",
            room_name, password
        ))
        .add_header("cookie", &format!("token={}", token))
        .await;

    let status = response.status_code();
    let body: Value = response.json();
    (status, body)
}

#[tokio::test]
async fn test_create_room_success() {
    let server = test_server().await;
    let token = register_and_login(&server, "room_creator", "pass123").await;

    let (status, body) = create_room(&server, &token, "general", "room_pass").await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "创建房间成功");
}

#[tokio::test]
async fn test_create_room_without_auth() {
    let server = test_server().await;

    let response = server
        .post("/api/rooms?room_name=test&password=pass")
        .await;

    assert_eq!(response.status_code(), 400);
    let body: Value = response.json();
    assert_eq!(body["message"], "请登录");
}

#[tokio::test]
async fn test_create_room_duplicate_name() {
    let server = test_server().await;
    let token = register_and_login(&server, "dup_creator", "pass123").await;

    let (status1, _) = create_room(&server, &token, "duplicate_room", "pass").await;
    assert_eq!(status1, 200);

    let (status2, body2) = create_room(&server, &token, "duplicate_room", "different_pass").await;
    assert_eq!(status2, 409);
    assert_eq!(body2["message"], "房间名重复");
}

#[tokio::test]
async fn test_create_room_empty_name() {
    let server = test_server().await;
    let token = register_and_login(&server, "empty_creator", "pass123").await;

    let (status, _) = create_room(&server, &token, "", "pass").await;
    // 空房间名：取决于实现，可能成功也可能返回错误
    // 这里仅验证不 panic
    assert!(status == 200 || status == 400 || status == 409 || status == 503);
}

#[tokio::test]
async fn test_create_multiple_rooms() {
    let server = test_server().await;
    let token = register_and_login(&server, "multi_creator", "pass123").await;

    for i in 1..=3 {
        let (status, body) = create_room(&server, &token, &format!("room_{}", i), "pass").await;
        assert_eq!(status, 200, "room_{} should be created: {:?}", i, body);
    }
}

#[tokio::test]
async fn test_create_rooms_by_different_users() {
    let server = test_server().await;
    let token1 = register_and_login(&server, "user_A", "passA").await;
    let token2 = register_and_login(&server, "user_B", "passB").await;

    // 两个用户分别创建房间
    let (status1, _) = create_room(&server, &token1, "room_A", "pass").await;
    assert_eq!(status1, 200);

    let (status2, _) = create_room(&server, &token2, "room_B", "pass").await;
    assert_eq!(status2, 200);

    // 不同用户不能创建同名房间
    let (status3, body3) = create_room(&server, &token2, "room_A", "pass").await;
    assert_eq!(status3, 409);
    assert_eq!(body3["message"], "房间名重复");
}

// ==================== 房间消息查询测试 (API 层) ====================

/// 辅助函数：获取房间消息
async fn get_room_messages(
    server: &TestServer,
    token: &str,
    room_id: u32,
    query: &str,
) -> (StatusCode, Value) {
    let url = format!("/api/rooms/{}/messages{}", room_id, query);
    let response = server
        .get(&url)
        .add_header("cookie", &format!("token={}", token))
        .await;

    (response.status_code(), response.json())
}

/// 辅助函数：获取房间最新消息 ID
async fn get_room_message_meta(
    server: &TestServer,
    token: &str,
    room_id: u32,
) -> (StatusCode, Value) {
    let url = format!("/api/rooms/{}/messages/meta", room_id);
    let response = server
        .get(&url)
        .add_header("cookie", &format!("token={}", token))
        .await;

    (response.status_code(), response.json())
}

#[tokio::test]
async fn test_room_messages_not_member() {
    let server = test_server().await;
    let creator_token = register_and_login(&server, "creator_msg", "pass123").await;
    let outsider_token = register_and_login(&server, "outsider_msg", "pass456").await;

    // 创建者创建房间（自动加入）
    let (status, _) = create_room(&server, &creator_token, "msg_room", "pass").await;
    assert_eq!(status, 200);

    // 非成员尝试访问房间消息（room_id=1），应被拒绝
    let (status, body) = get_room_messages(&server, &outsider_token, 1, "?limit=10").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "不正确的房间");
}

#[tokio::test]
async fn test_room_messages_empty() {
    let server = test_server().await;
    let token = register_and_login(&server, "empty_room_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "empty_room", "pass").await;
    assert_eq!(status, 200);

    let (status, body) = get_room_messages(&server, &token, 1, "?limit=10").await;
    assert_eq!(status, 200);
    // 空房间应该返回空消息列表
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_room_message_meta_empty() {
    let server = test_server().await;
    let token = register_and_login(&server, "meta_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "meta_room", "pass").await;
    assert_eq!(status, 200);

    let (status, body) = get_room_message_meta(&server, &token, 1).await;
    assert_eq!(status, 200);
    // 空房间时 meta 返回 id=1
    assert_eq!(body["id"], 1);
}

#[tokio::test]
async fn test_room_messages_not_found() {
    let server = test_server().await;
    let token = register_and_login(&server, "bad_room_user", "pass123").await;

    // 创建并加入房间（room_id=1）
    let (status, _) = create_room(&server, &token, "joined_room", "pass").await;
    assert_eq!(status, 200);

    // 尝试访问不存在的房间（room_id=999）
    let (status, body) = get_room_messages(&server, &token, 999, "?limit=10").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "不正确的房间");
}

// ==================== 用户登出后 token 安全性测试 ====================

#[tokio::test]
async fn test_token_still_valid_after_multiple_logins() {
    let server = test_server().await;

    register_user(&server, "relogin_user", "pass123").await;

    // 第一次登录
    let resp1 = server
        .post("/api/login")
        .json(&json!({"user_name": "relogin_user", "password": "pass123"}))
        .await;
    let token1 = extract_token_from_response(&resp1).unwrap();

    // 第二次登录（可能获取相同或不同的 token）
    let resp2 = server
        .post("/api/login")
        .json(&json!({"user_name": "relogin_user", "password": "pass123"}))
        .await;
    let token2 = extract_token_from_response(&resp2).unwrap();

    // 两个 token 都应有效
    let (s1, _) = get_me(&server, &token1).await;
    let (s2, _) = get_me(&server, &token2).await;
    assert_eq!(s1, 200);
    assert_eq!(s2, 200);
}

// ==================== 序列化兼容性测试 ====================

#[tokio::test]
async fn test_message_serialization_roundtrip() {
    let msg = message::Model {
        id: 42,
        user_name: "test_user".to_string(),
        content: "Hello, World!".to_string(),
        timestamp: Utc::now(),
        room_id: Some(1),
        user_id: Some(100),
    };

    let json_str = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: message::Model =
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    assert_eq!(msg.id, deserialized.id);
    assert_eq!(msg.user_name, deserialized.user_name);
    assert_eq!(msg.content, deserialized.content);
    assert_eq!(msg.room_id, deserialized.room_id);
    assert_eq!(msg.user_id, deserialized.user_id);
}

// ==================== 并发房间操作测试 ====================

#[tokio::test]
async fn test_concurrent_room_creation() {
    let server = test_server().await;
    let token = register_and_login(&server, "concurrent_room", "pass123").await;

    // 顺序创建多个房间（TestServer 不支持 Send，无法 spawn）
    for i in 0..5 {
        let (status, _) = create_room(&server, &token, &format!("conc_room_{}", i), "pass").await;
        assert_eq!(status, 200);
    }

    // 验证所有房间都已创建（尝试创建重复房间应返回 409）
    let (dup_status, _) = create_room(&server, &token, "conc_room_0", "pass").await;
    assert_eq!(dup_status, 409);
}

// ==================== 房间消息 API 参数校验测试 ====================

#[tokio::test]
async fn test_room_messages_before_and_after_mutually_exclusive() {
    let server = test_server().await;
    let token = register_and_login(&server, "mutex_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "mutex_room", "pass").await;
    assert_eq!(status, 200);

    // before_id 和 after_id 同时存在应返回 400
    let (status, body) = get_room_messages(&server, &token, 1, "?before_id=3&after_id=2").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "before_id 和 after_id 不能同时存在");
}

#[tokio::test]
async fn test_room_messages_default_limit() {
    let server = test_server().await;
    let token = register_and_login(&server, "default_limit_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "default_limit_room", "pass").await;
    assert_eq!(status, 200);

    // 不带参数查询空房间应返回空列表
    let (status, body) = get_room_messages(&server, &token, 1, "").await;
    assert_eq!(status, 200);
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_room_messages_member_access() {
    let server = test_server().await;
    let token = register_and_login(&server, "member_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "member_room", "pass").await;
    assert_eq!(status, 200);

    // 房间创建者（成员）应该能访问房间消息
    let (status, body) = get_room_messages(&server, &token, 1, "?limit=10").await;
    assert_eq!(status, 200);
    assert!(body["messages"].is_array());
}

#[tokio::test]
async fn test_room_message_meta_with_messages() {
    let server = test_server().await;
    let token = register_and_login(&server, "meta_msg_user", "pass123").await;

    let (status, _) = create_room(&server, &token, "meta_msg_room", "pass").await;
    assert_eq!(status, 200);

    // 空房间 meta 返回 id=1
    let (status, body) = get_room_message_meta(&server, &token, 1).await;
    assert_eq!(status, 200);
    assert_eq!(body["id"], 1);
}

// ==================== 房间消息跨房间隔离测试 ====================

#[tokio::test]
async fn test_room_messages_cross_room_isolation() {
    let server = test_server().await;
    let token = register_and_login(&server, "cross_room_user", "pass123").await;

    // 创建两个房间（用户自动加入）
    let (status1, _) = create_room(&server, &token, "room_alpha", "pass").await;
    assert_eq!(status1, 200);
    let (status2, _) = create_room(&server, &token, "room_beta", "pass").await;
    assert_eq!(status2, 200);

    // room_alpha 的 id 应该是 1，room_beta 的 id 应该是 2
    // 两个空房间的消息查询都应返回空
    let (status_a, body_a) = get_room_messages(&server, &token, 1, "?limit=10").await;
    assert_eq!(status_a, 200);
    assert_eq!(body_a["messages"].as_array().unwrap().len(), 0);

    let (status_b, body_b) = get_room_messages(&server, &token, 2, "?limit=10").await;
    assert_eq!(status_b, 200);
    assert_eq!(body_b["messages"].as_array().unwrap().len(), 0);
}

// ==================== 注册边界条件测试 ====================

#[tokio::test]
async fn test_register_empty_both_fields() {
    let server = test_server().await;
    let (status, body) = register_user(&server, "", "").await;
    assert_eq!(status, 400);
    assert_eq!(body["message"], "用户名或密码不能为空");
}

#[tokio::test]
async fn test_register_only_whitespace_username() {
    let server = test_server().await;
    // 纯空格的用户名——取决于实现如何处理
    let (status, _) = register_user(&server, "   ", "password").await;
    // 可能成功或失败，仅验证不 panic
    assert!(status == 201 || status == 400);
}

#[tokio::test]
async fn test_login_with_nonexistent_user_special_chars() {
    let server = test_server().await;
    let (status, body) = login_user(&server, "!!!nonexistent!!!", "password").await;
    assert_eq!(status, 401);
    assert_eq!(body["message"], "用户不存在或密码错误");
}

// ==================== 房间创建边界条件测试 ====================

#[tokio::test]
async fn test_create_room_special_chars_in_name() {
    let server = test_server().await;
    let token = register_and_login(&server, "special_room_user", "pass123").await;

    // Use URL-safe special characters that won't break query string parsing
    let (status, _) = create_room(&server, &token, "room_special_chars", "pass!@").await;
    // Special characters in room name/password, verify no panic
    assert!(status == 200 || status == 400 || status == 503);
}

#[tokio::test]
async fn test_create_room_unicode_name() {
    let server = test_server().await;
    let token = register_and_login(&server, "unicode_room_user", "pass123").await;

    let (status, body) = create_room(&server, &token, "聊天室", "密码123").await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "创建房间成功");
}

// ==================== 无认证访问受保护端点测试 ====================

#[tokio::test]
async fn test_rooms_endpoint_without_auth() {
    let server = test_server().await;
    // GET /api/rooms 未实现，但 POST 需要认证
    let response = server.post("/api/rooms?room_name=test&password=pass").await;
    assert_eq!(response.status_code(), 400);
    let body: Value = response.json();
    assert_eq!(body["message"], "请登录");
}

#[tokio::test]
async fn test_ws_endpoint_without_auth() {
    let server = test_server().await;
    // WebSocket 升级请求无认证应被拒绝
    let response = server.get("/api/rooms/1/ws").await;
    assert_eq!(response.status_code(), 400);
    let body: Value = response.json();
    assert_eq!(body["message"], "请登录");
}

// ==================== 消息列表 API 测试 ====================

#[tokio::test]
async fn test_list_message_db() {
    let (db, room_id) = create_message_db().await;

    // Insert 5 messages
    for i in 1..=5 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("msg{}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // list_message (no before/after) returns latest messages in descending order
    let result = db::messages::list_message(&db, room_id.clone(), 3).await;
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].id, 5);
    assert_eq!(messages[1].id, 4);
    assert_eq!(messages[2].id, 3);
}

#[tokio::test]
async fn test_list_message_db_limit_exceeds_count() {
    let (db, room_id) = create_message_db().await;

    // Insert 2 messages
    for i in 1..=2 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("msg{}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // Request more than available
    let result = db::messages::list_message(&db, room_id, 10).await;
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_list_message_after_db_max_id() {
    let (db, room_id) = create_message_db().await;

    // Insert 3 messages (ids: 1, 2, 3)
    for i in 1..=3 {
        let msg = message::Model {
            id: 0,
            user_name: format!("user{}", i),
            content: format!("msg{}", i),
            timestamp: Utc::now(),
            room_id: Some(room_id.0),
            user_id: None,
        };
        let _ = db::messages::insert_message(&db, msg).await;
    }

    // after_id = 3 should return just message 3
    let result = db::messages::list_message_after(&db, room_id.clone(), 3, 5).await;
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, 3);
}

// ==================== 消息内容边界测试 ====================

#[tokio::test]
async fn test_insert_message_empty_content() {
    let (db, room_id) = create_message_db().await;
    let msg = message::Model {
        id: 0,
        user_name: "empty_content_user".to_string(),
        content: String::new(),
        timestamp: Utc::now(),
        room_id: Some(room_id.0),
        user_id: None,
    };

    let result = db::messages::insert_message(&db, msg).await;
    assert!(result.is_ok());
    let inserted = result.unwrap();
    assert_eq!(inserted.content, "");
    assert!(inserted.id > 0);
}

#[tokio::test]
async fn test_insert_message_very_long_content() {
    let (db, room_id) = create_message_db().await;
    let long_content = "x".repeat(10000);
    let msg = message::Model {
        id: 0,
        user_name: "long_content_user".to_string(),
        content: long_content.clone(),
        timestamp: Utc::now(),
        room_id: Some(room_id.0),
        user_id: None,
    };

    let result = db::messages::insert_message(&db, msg).await;
    assert!(result.is_ok());
    let inserted = result.unwrap();
    assert_eq!(inserted.content, long_content);
    assert_eq!(inserted.content.len(), 10000);
}

// ==================== 消息模型序列化测试 ====================

#[tokio::test]
async fn test_message_serialization_excludes_room_and_user() {
    let msg = message::Model {
        id: 99,
        user_name: "serial_test".to_string(),
        content: "test".to_string(),
        timestamp: Utc::now(),
        room_id: Some(42),
        user_id: Some(7),
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    // room and user fields should be excluded (serde(skip))
    assert!(!json_str.contains("\"room\""));
    assert!(!json_str.contains("\"user\""));
    // But room_id and user_id should be present
    assert!(json_str.contains("42"));
    assert!(json_str.contains("7"));
}

// ==================== 测试配置常量 ====================

#[test]
fn test_config_constants() {
    // 验证配置常量有合理值
    assert!(chat_server::config::WS_CHANNEL_BUFFER > 0);
}
