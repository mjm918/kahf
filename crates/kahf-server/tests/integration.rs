//! Integration tests for the kahf-server HTTP API.
//!
//! Tests exercise the full request/response cycle against the staging
//! database using axum's built-in test utilities. Each test creates a
//! fresh JWT secret to isolate token namespaces. Tests cover auth flows,
//! user profile, workspace management, entity CRUD with event sourcing,
//! time-travel history, error responses, and the health endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use kahf_auth::JwtConfig;
use kahf_db::DbPool;
use kahf_realtime::{BroadcastEventBus, Hub};
use kahf_server::app_state::AppState;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

fn database_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment")
}

async fn make_state(db: DbPool, jwt: JwtConfig) -> AppState {
    let hub = Hub::new(db.pool().clone());
    let event_bus = BroadcastEventBus::new(64);
    let rbac = kahf_rbac::RbacEnforcer::new(&database_url()).await.unwrap();
    AppState::new(db, jwt, hub, event_bus, rbac)
}

async fn assign_owner(state: &AppState, user_id: uuid::Uuid, workspace_id: uuid::Uuid) {
    kahf_rbac::assign_role(&state.rbac, user_id, "owner", workspace_id)
        .await
        .unwrap();
}

async fn test_app() -> axum::Router {
    let db = DbPool::connect(&database_url()).await.unwrap();
    db.migrate().await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let state = make_state(db, jwt.clone()).await;
    kahf_server::build_app(state, jwt)
}

fn unique_email() -> String {
    format!("test-{}@kahf.test", Uuid::new_v4())
}

fn unique_slug() -> String {
    format!("test-{}", &Uuid::new_v4().to_string()[..8])
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn signup(app: &axum::Router, email: &str, password: &str, name: &str) -> Value {
    let body = serde_json::json!({ "email": email, "password": password, "name": name });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await
}

async fn login(app: &axum::Router, email: &str, password: &str) -> Value {
    let body = serde_json::json!({ "email": email, "password": password });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    body_json(resp).await
}

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

#[tokio::test]
async fn test_health_check() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["postgres"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_signup_creates_user() {
    let app = test_app().await;
    let email = unique_email();
    let json = signup(&app, &email, "StrongPass1!", "Test User").await;

    assert!(json["access_token"].is_string());
    assert!(json["refresh_token"].is_string());
    assert_eq!(json["email"], email);
    assert_eq!(json["name"], "Test User");
    assert!(json["user_id"].is_string());
}

#[tokio::test]
async fn test_signup_duplicate_email_fails() {
    let app = test_app().await;
    let email = unique_email();
    signup(&app, &email, "StrongPass1!", "User One").await;

    let body = serde_json::json!({ "email": email, "password": "StrongPass1!", "name": "User Two" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_login_success() {
    let app = test_app().await;
    let email = unique_email();
    signup(&app, &email, "MyPassword1!", "Login User").await;
    let json = login(&app, &email, "MyPassword1!").await;

    assert!(json["access_token"].is_string());
    assert!(json["refresh_token"].is_string());
    assert_eq!(json["email"], email);
}

#[tokio::test]
async fn test_login_wrong_password() {
    let app = test_app().await;
    let email = unique_email();
    signup(&app, &email, "CorrectPass1!", "Wrong Pass").await;

    let body = serde_json::json!({ "email": email, "password": "WrongPass1!" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let app = test_app().await;
    let body = serde_json::json!({ "email": "nonexistent@kahf.test", "password": "Whatever1!" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "RefreshMe1!", "Refresh User").await;
    let refresh_token = signup_json["refresh_token"].as_str().unwrap();

    let body = serde_json::json!({ "refresh_token": refresh_token });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["access_token"].is_string());
}

#[tokio::test]
async fn test_refresh_with_invalid_token() {
    let app = test_app().await;
    let body = serde_json::json!({ "refresh_token": "invalid.token.here" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_server_error() || resp.status().is_client_error());
}

#[tokio::test]
async fn test_logout() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_me() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "GetMe1234!", "Me User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .header("authorization", auth_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["email"], email);
    assert_eq!(json["name"], "Me User");
}

#[tokio::test]
async fn test_get_me_without_auth() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_me() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "UpdateMe1!", "Old Name").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let body = serde_json::json!({ "name": "New Name", "avatar_url": "https://example.com/avatar.png" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["name"], "New Name");
    assert_eq!(json["avatar_url"], "https://example.com/avatar.png");
}

#[tokio::test]
async fn test_create_workspace() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "WorkspacePass1!", "WS User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let slug = unique_slug();
    let body = serde_json::json!({ "name": "Test Workspace", "slug": slug });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["name"], "Test Workspace");
    assert_eq!(json["slug"], slug);
}

#[tokio::test]
async fn test_list_workspaces() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "ListWS1!", "List User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let slug = unique_slug();
    let body = serde_json::json!({ "name": "Listed WS", "slug": slug });
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().unwrap();
    assert!(arr.iter().any(|w| w["slug"] == slug));
}

#[tokio::test]
async fn test_get_workspace_by_id() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "GetWS1!", "Get WS User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let slug = unique_slug();
    let body = serde_json::json!({ "name": "Get WS", "slug": slug });
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_json = body_json(create_resp).await;
    let ws_id = create_json["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{ws_id}"))
                .header("authorization", auth_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["slug"], slug);
}

#[tokio::test]
async fn test_get_nonexistent_workspace() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "NoWS1!", "No WS User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let fake_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{fake_id}"))
                .header("authorization", auth_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_workspace_add_and_remove_member() {
    let app = test_app().await;

    let email1 = unique_email();
    let signup1 = signup(&app, &email1, "Owner1!", "Owner").await;
    let token1 = signup1["access_token"].as_str().unwrap();
    let user1_id = signup1["user_id"].as_str().unwrap();

    let email2 = unique_email();
    let signup2 = signup(&app, &email2, "Member1!", "Member").await;
    let user2_id = signup2["user_id"].as_str().unwrap();

    let slug = unique_slug();
    let ws_body = serde_json::json!({ "name": "Member WS", "slug": slug });
    let ws_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&ws_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ws_json = body_json(ws_resp).await;
    let ws_id = ws_json["id"].as_str().unwrap();

    let add_body = serde_json::json!({ "user_id": user2_id, "role": "member" });
    let add_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workspaces/{ws_id}/members"))
                .header("authorization", auth_header(token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&add_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(add_resp.status(), StatusCode::CREATED);

    let remove_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{ws_id}/members/{user2_id}"))
                .header("authorization", auth_header(token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remove_resp.status(), StatusCode::NO_CONTENT);

    let _ = user1_id;
}

#[tokio::test]
async fn test_create_entity() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "Entity1!", "Entity User").await;
    let user_id = signup_json["user_id"].as_str().unwrap();

    let slug = unique_slug();
    let ws_body = serde_json::json!({ "name": "Entity WS", "slug": slug });

    let jwt_secret = format!("test-secret-{}", Uuid::new_v4());
    let jwt = JwtConfig::new(jwt_secret);

    let db = DbPool::connect(&database_url()).await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(
        db.pool(),
        "Entity WS",
        &slug,
        uuid::Uuid::parse_str(user_id).unwrap(),
    )
    .await
    .unwrap();

    let access_token = kahf_auth::jwt::issue_access_token(
        &jwt,
        uuid::Uuid::parse_str(user_id).unwrap(),
        Some(ws.id),
        Some("owner".into()),
    )
    .unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, uuid::Uuid::parse_str(user_id).unwrap(), ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    let task_data = serde_json::json!({
        "title": "Test Task",
        "status": "todo",
        "priority": "high"
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/entities/task")
                .header("authorization", auth_header(&access_token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&task_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["title"], "Test Task");
    assert_eq!(json["data"]["status"], "todo");
    assert!(!json["deleted"].as_bool().unwrap());

    let _ = ws_body;
}

#[tokio::test]
async fn test_get_entity() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let user_id = Uuid::new_v4();
    kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "Get Entity User").await.unwrap();

    let slug = unique_slug();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));

    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "Get Entity User 2").await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "Get Entity WS", &slug, user.id).await.unwrap();

    let access_token = kahf_auth::jwt::issue_access_token(
        &jwt,
        user.id,
        Some(ws.id),
        Some("owner".into()),
    )
    .unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, user.id, ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    let task_data = serde_json::json!({ "title": "Fetch Me" });
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/entities/task")
                .header("authorization", auth_header(&access_token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&task_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_json = body_json(create_resp).await;
    let entity_id = create_json["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/entities/task/{entity_id}"))
                .header("authorization", auth_header(&access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["title"], "Fetch Me");

    let _ = user_id;
}

#[tokio::test]
async fn test_update_entity() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "Update Entity User").await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "Update Entity WS", &unique_slug(), user.id).await.unwrap();
    let token = kahf_auth::jwt::issue_access_token(&jwt, user.id, Some(ws.id), Some("owner".into())).unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, user.id, ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    let create_data = serde_json::json!({ "title": "Original", "status": "todo" });
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/entities/task")
                .header("authorization", auth_header(&token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_json = body_json(create_resp).await;
    let entity_id = create_json["id"].as_str().unwrap();

    let patch_data = serde_json::json!({ "status": "in_progress" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/entities/task/{entity_id}"))
                .header("authorization", auth_header(&token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&patch_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["title"], "Original");
    assert_eq!(json["data"]["status"], "in_progress");
}

#[tokio::test]
async fn test_delete_entity() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "Delete Entity User").await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "Delete Entity WS", &unique_slug(), user.id).await.unwrap();
    let token = kahf_auth::jwt::issue_access_token(&jwt, user.id, Some(ws.id), Some("owner".into())).unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, user.id, ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    let create_data = serde_json::json!({ "title": "Delete Me" });
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/entities/task")
                .header("authorization", auth_header(&token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_json = body_json(create_resp).await;
    let entity_id = create_json["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/entities/task/{entity_id}"))
                .header("authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_entity_history() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "History User").await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "History WS", &unique_slug(), user.id).await.unwrap();
    let token = kahf_auth::jwt::issue_access_token(&jwt, user.id, Some(ws.id), Some("owner".into())).unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, user.id, ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    let create_data = serde_json::json!({ "title": "History Task" });
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/entities/task")
                .header("authorization", auth_header(&token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_json = body_json(create_resp).await;
    let entity_id = create_json["id"].as_str().unwrap();

    let patch_data = serde_json::json!({ "status": "done" });
    app.clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/entities/task/{entity_id}"))
                .header("authorization", auth_header(&token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&patch_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/entities/task/{entity_id}/history"))
                .header("authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let events = json.as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["op"], "create");
    assert_eq!(events[1]["op"], "update");
}

#[tokio::test]
async fn test_list_entities() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "List Entity User").await.unwrap();
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "List Entity WS", &unique_slug(), user.id).await.unwrap();
    let token = kahf_auth::jwt::issue_access_token(&jwt, user.id, Some(ws.id), Some("owner".into())).unwrap();

    let state = make_state(db, jwt.clone()).await;
    assign_owner(&state, user.id, ws.id).await;
    let app = kahf_server::build_app(state, jwt);

    for i in 0..3 {
        let data = serde_json::json!({ "title": format!("Task {i}") });
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/entities/contact")
                    .header("authorization", auth_header(&token))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/entities/contact")
                .header("authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let entities = json.as_array().unwrap();
    assert!(entities.len() >= 3);
}

#[tokio::test]
async fn test_get_nonexistent_entity() {
    let db = DbPool::connect(&database_url()).await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let user = kahf_db::user_repo::create_user(db.pool(), &unique_email(), "$argon2id$v=19$m=19456,t=2,p=1$fake$fakehashvalue00000000000000000000", "NoEntity User").await.unwrap();
    let token = kahf_auth::jwt::issue_access_token(&jwt, user.id, None, None).unwrap();

    let state = make_state(db, jwt.clone()).await;
    let app = kahf_server::build_app(state, jwt);

    let fake_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/entities/task/{fake_id}"))
                .header("authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_signup_missing_fields() {
    let app = test_app().await;
    let body = serde_json::json!({ "email": "missing@fields.com" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_entity_requires_auth() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/entities/task")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_workspace_duplicate_slug_fails() {
    let app = test_app().await;
    let email = unique_email();
    let signup_json = signup(&app, &email, "DupSlug1!", "Dup Slug User").await;
    let token = signup_json["access_token"].as_str().unwrap();

    let slug = unique_slug();
    let body = serde_json::json!({ "name": "First WS", "slug": slug });

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body2 = serde_json::json!({ "name": "Second WS", "slug": slug });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("authorization", auth_header(token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn start_test_server() -> (String, JwtConfig, sqlx::PgPool) {
    let db = DbPool::connect(&database_url()).await.unwrap();
    db.migrate().await.unwrap();
    let jwt = JwtConfig::new(format!("test-secret-{}", Uuid::new_v4()));
    let pool = db.pool().clone();
    let state = make_state(db, jwt.clone()).await;
    let app = kahf_server::build_app(state, jwt.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, jwt, pool)
}

async fn ws_signup_and_create_workspace(
    addr: &str,
    jwt: &JwtConfig,
    email: &str,
) -> (String, Uuid) {
    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let resp: Value = client
        .post(format!("{}/api/auth/signup", base))
        .json(&serde_json::json!({
            "email": email,
            "password": "TestPass1!",
            "name": "WS Test User"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let access_token = resp["access_token"].as_str().unwrap().to_string();

    let ws_resp: Value = client
        .post(format!("{}/api/workspaces", base))
        .header("authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({
            "name": "WS Test Workspace",
            "slug": unique_slug()
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let workspace_id =
        Uuid::parse_str(ws_resp["id"].as_str().unwrap()).unwrap();

    let claims = kahf_auth::jwt::verify_token(jwt, &access_token).unwrap();
    let ws_token = kahf_auth::jwt::issue_access_token(
        jwt,
        claims.sub,
        Some(workspace_id),
        None,
    )
    .unwrap();

    (ws_token, workspace_id)
}

#[tokio::test]
async fn test_ws_connect_with_valid_token() {
    let (addr, jwt, _pool) = start_test_server().await;
    let email = unique_email();
    let (token, _ws_id) = ws_signup_and_create_workspace(&addr, &jwt, &email).await;

    let url = format!("ws://{}/ws?token={}", addr, token);
    let (ws_stream, resp) =
        tokio_tungstenite::connect_async(&url).await.unwrap();
    assert_eq!(resp.status(), 101);
    drop(ws_stream);
}

#[tokio::test]
async fn test_ws_connect_without_token_fails() {
    let (addr, _jwt, _pool) = start_test_server().await;

    let url = format!("ws://{}/ws", addr);
    let result = tokio_tungstenite::connect_async(&url).await;
    assert!(result.is_err() || {
        let (_ws, resp) = result.unwrap();
        resp.status() != 101
    });
}

#[tokio::test]
async fn test_ws_connect_with_invalid_token_fails() {
    let (addr, _jwt, _pool) = start_test_server().await;

    let url = format!("ws://{}/ws?token=invalid-jwt-token", addr);
    let result = tokio_tungstenite::connect_async(&url).await;
    assert!(result.is_err() || {
        let (_ws, resp) = result.unwrap();
        resp.status() != 101
    });
}

async fn ws_signup_and_join_workspace(
    addr: &str,
    jwt: &JwtConfig,
    email: &str,
    workspace_id: Uuid,
    owner_token: &str,
) -> (String, Uuid) {
    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let resp: Value = client
        .post(format!("{}/api/auth/signup", base))
        .json(&serde_json::json!({
            "email": email,
            "password": "TestPass1!",
            "name": "WS Test User 2"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let access_token = resp["access_token"].as_str().unwrap().to_string();
    let claims = kahf_auth::jwt::verify_token(jwt, &access_token).unwrap();
    let user_id = claims.sub;

    client
        .post(format!("{}/api/workspaces/{}/members", base, workspace_id))
        .header("authorization", format!("Bearer {}", owner_token))
        .json(&serde_json::json!({
            "user_id": user_id,
            "role": "member"
        }))
        .send()
        .await
        .unwrap();

    let ws_token = kahf_auth::jwt::issue_access_token(
        jwt,
        user_id,
        Some(workspace_id),
        None,
    )
    .unwrap();

    (ws_token, user_id)
}

#[tokio::test]
async fn test_ws_presence_broadcast() {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;

    let email1 = unique_email();
    let (token1, ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email1).await;

    let url1 = format!("ws://{}/ws?token={}", addr, token1);
    let (mut ws1, _) = tokio_tungstenite::connect_async(&url1).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let email2 = unique_email();
    let (token2, _user2_id) =
        ws_signup_and_join_workspace(&addr, &jwt, &email2, ws_id, &token1).await;

    let url2 = format!("ws://{}/ws?token={}", addr, token2);
    let (_ws2, _) = tokio_tungstenite::connect_async(&url2).await.unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws1.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "presence.update" {
                    return val;
                }
            }
        }
        panic!("never received presence.update");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for presence.update");
    let val = timeout.unwrap();
    assert!(val["user"].is_string());
    assert_eq!(val["status"], "online");
}

#[tokio::test]
async fn test_ws_chat_message_broadcast() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;

    let email1 = unique_email();
    let (token1, ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email1).await;

    let url1 = format!("ws://{}/ws?token={}", addr, token1);
    let (mut ws1, _) = tokio_tungstenite::connect_async(&url1).await.unwrap();

    let email2 = unique_email();
    let (token2, _user2_id) =
        ws_signup_and_join_workspace(&addr, &jwt, &email2, ws_id, &token1).await;
    let url2 = format!("ws://{}/ws?token={}", addr, token2);
    let (mut ws2, _) = tokio_tungstenite::connect_async(&url2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let channel_id = Uuid::new_v4();
    let chat_msg = serde_json::json!({
        "type": "chat.message",
        "channel_id": channel_id,
        "text": "hello from ws2",
        "user": Uuid::new_v4()
    });
    ws2.send(Message::Text(chat_msg.to_string().into()))
        .await
        .unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws1.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "chat.message" {
                    return val;
                }
            }
        }
        panic!("never received chat.message");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for chat.message");
    let val = timeout.unwrap();
    assert_eq!(val["text"], "hello from ws2");
}

#[tokio::test]
async fn test_ws_crdt_join_returns_state() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;

    let email = unique_email();
    let (token, _ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email).await;
    let url = format!("ws://{}/ws?token={}", addr, token);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let doc_id = Uuid::new_v4();
    let join_msg = serde_json::json!({
        "type": "crdt.join",
        "doc_id": doc_id.to_string()
    });
    ws.send(Message::Text(join_msg.to_string().into()))
        .await
        .unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "crdt.state" {
                    return val;
                }
            }
        }
        panic!("never received crdt.state");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for crdt.state");
    let val = timeout.unwrap();
    assert_eq!(val["doc_id"], doc_id.to_string());
    assert!(val["payload"].is_string());
}

#[tokio::test]
async fn test_ws_entity_created_broadcast() {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;
    let email = unique_email();
    let (token, _ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email).await;

    let url = format!("ws://{}/ws?token={}", addr, token);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);
    let create_resp: Value = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "WS broadcast test",
            "status": "open"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let created_id = create_resp["id"].as_str().unwrap().to_string();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "entity.created" {
                    return val;
                }
            }
        }
        panic!("never received entity.created");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for entity.created");
    let val = timeout.unwrap();
    assert_eq!(val["entity_type"], "task");
    assert_eq!(val["id"], created_id);
    assert_eq!(val["data"]["title"], "WS broadcast test");
}

#[tokio::test]
async fn test_ws_entity_updated_broadcast() {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;
    let email = unique_email();
    let (token, _ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let create_resp: Value = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Before update",
            "status": "open"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let entity_id = create_resp["id"].as_str().unwrap().to_string();

    let url = format!("ws://{}/ws?token={}", addr, token);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    client
        .patch(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "status": "done" }))
        .send()
        .await
        .unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "entity.updated" {
                    return val;
                }
            }
        }
        panic!("never received entity.updated");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for entity.updated");
    let val = timeout.unwrap();
    assert_eq!(val["entity_type"], "task");
    assert_eq!(val["id"], entity_id);
    assert_eq!(val["patch"]["status"], "done");
}

#[tokio::test]
async fn test_ws_entity_deleted_broadcast() {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message;

    let (addr, jwt, _pool) = start_test_server().await;
    let email = unique_email();
    let (token, _ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let create_resp: Value = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "title": "To be deleted" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let entity_id = create_resp["id"].as_str().unwrap().to_string();

    let url = format!("ws://{}/ws?token={}", addr, token);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    client
        .delete(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "entity.deleted" {
                    return val;
                }
            }
        }
        panic!("never received entity.deleted");
    })
    .await;

    assert!(timeout.is_ok(), "timed out waiting for entity.deleted");
    let val = timeout.unwrap();
    assert_eq!(val["entity_type"], "task");
    assert_eq!(val["id"], entity_id);
}

#[tokio::test]
async fn test_rbac_owner_can_manage_entities() {
    let (addr, jwt, _pool) = start_test_server().await;
    let email = unique_email();
    let (token, _ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &email).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let resp = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "title": "Owner task" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let created: Value = resp.json().await.unwrap();
    let entity_id = created["id"].as_str().unwrap();

    let resp = client
        .patch(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "status": "done" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .delete(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_rbac_member_can_read_create_update_but_not_delete() {
    let (addr, jwt, _pool) = start_test_server().await;

    let owner_email = unique_email();
    let (owner_token, ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &owner_email).await;

    let member_email = unique_email();
    let (member_token, _member_id) =
        ws_signup_and_join_workspace(&addr, &jwt, &member_email, ws_id, &owner_token).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let resp = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", member_token))
        .json(&serde_json::json!({ "title": "Member task" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let created: Value = resp.json().await.unwrap();
    let entity_id = created["id"].as_str().unwrap();

    let resp = client
        .get(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .patch(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", member_token))
        .json(&serde_json::json!({ "status": "done" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .delete(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn test_rbac_guest_can_only_read() {
    let (addr, jwt, _pool) = start_test_server().await;

    let owner_email = unique_email();
    let (owner_token, ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &owner_email).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);

    let resp = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", owner_token))
        .json(&serde_json::json!({ "title": "Owner's task" }))
        .send()
        .await
        .unwrap();
    let created: Value = resp.json().await.unwrap();
    let entity_id = created["id"].as_str().unwrap();

    let guest_email = unique_email();
    let guest_resp: Value = client
        .post(format!("{}/api/auth/signup", base))
        .json(&serde_json::json!({
            "email": guest_email,
            "password": "GuestPass1!",
            "name": "Guest User"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let guest_access = guest_resp["access_token"].as_str().unwrap();
    let guest_claims = kahf_auth::jwt::verify_token(&jwt, guest_access).unwrap();

    client
        .post(format!("{}/api/workspaces/{}/members", base, ws_id))
        .header("authorization", format!("Bearer {}", owner_token))
        .json(&serde_json::json!({
            "user_id": guest_claims.sub,
            "role": "guest"
        }))
        .send()
        .await
        .unwrap();

    let guest_ws_token = kahf_auth::jwt::issue_access_token(
        &jwt,
        guest_claims.sub,
        Some(ws_id),
        None,
    )
    .unwrap();

    let resp = client
        .get(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", guest_ws_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .post(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", guest_ws_token))
        .json(&serde_json::json!({ "title": "Guest attempt" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = client
        .delete(format!("{}/api/entities/task/{}", base, entity_id))
        .header("authorization", format!("Bearer {}", guest_ws_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn test_rbac_no_role_gets_forbidden() {
    let (addr, jwt, _pool) = start_test_server().await;

    let owner_email = unique_email();
    let (_owner_token, ws_id) =
        ws_signup_and_create_workspace(&addr, &jwt, &owner_email).await;

    let outsider_email = unique_email();
    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);
    let resp: Value = client
        .post(format!("{}/api/auth/signup", base))
        .json(&serde_json::json!({
            "email": outsider_email,
            "password": "Outsider1!",
            "name": "Outsider"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let outsider_claims = kahf_auth::jwt::verify_token(&jwt, resp["access_token"].as_str().unwrap()).unwrap();
    let outsider_token = kahf_auth::jwt::issue_access_token(
        &jwt,
        outsider_claims.sub,
        Some(ws_id),
        None,
    )
    .unwrap();

    let resp = client
        .get(format!("{}/api/entities/task", base))
        .header("authorization", format!("Bearer {}", outsider_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}
