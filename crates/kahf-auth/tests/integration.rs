//! Integration tests for kahf-auth against the staging database.
//!
//! Tests cover password hashing, JWT token lifecycle, auth service
//! operations (signup, login, refresh), and error paths including
//! wrong passwords, expired tokens, duplicate emails, and invalid
//! token types.

use chrono::Duration;
use kahf_auth::jwt::{JwtConfig, issue_access_token, issue_refresh_token, verify_token};
use kahf_auth::password::{hash_password, verify_password};
use kahf_auth::service;
use kahf_db::pool::DbPool;
use uuid::Uuid;

fn database_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment")
}

fn test_jwt_config() -> JwtConfig {
    JwtConfig::new("test-secret-key-for-integration-tests".to_string())
}

async fn test_pool() -> DbPool {
    DbPool::connect(&database_url()).await.expect("failed to connect to staging DB")
}

#[test]
fn password_hash_and_verify() {
    let hash = hash_password("my-secure-password").unwrap();
    assert!(hash.starts_with("$argon2id$"), "should produce argon2id PHC string");
    verify_password("my-secure-password", &hash).unwrap();
}

#[test]
fn password_verify_wrong_password_fails() {
    let hash = hash_password("correct-password").unwrap();
    let result = verify_password("wrong-password", &hash);
    assert!(result.is_err(), "wrong password should fail verification");
}

#[test]
fn password_verify_invalid_hash_format_fails() {
    let result = verify_password("password", "not-a-valid-hash");
    assert!(result.is_err(), "invalid hash format should fail");
}

#[test]
fn password_different_hashes_for_same_password() {
    let hash1 = hash_password("same-password").unwrap();
    let hash2 = hash_password("same-password").unwrap();
    assert_ne!(hash1, hash2, "different salts should produce different hashes");
}

#[test]
fn jwt_issue_and_verify_access_token() {
    let config = test_jwt_config();
    let user_id = Uuid::new_v4();

    let token = issue_access_token(&config, user_id, None, None).unwrap();
    let claims = verify_token(&config, &token).unwrap();

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.token_type, "access");
    assert!(claims.workspace_id.is_none());
}

#[test]
fn jwt_issue_access_token_with_workspace_and_role() {
    let config = test_jwt_config();
    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();

    let token = issue_access_token(&config, user_id, Some(workspace_id), Some("admin".to_string())).unwrap();
    let claims = verify_token(&config, &token).unwrap();

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.workspace_id, Some(workspace_id));
    assert_eq!(claims.role.as_deref(), Some("admin"));
}

#[test]
fn jwt_issue_and_verify_refresh_token() {
    let config = test_jwt_config();
    let user_id = Uuid::new_v4();

    let token = issue_refresh_token(&config, user_id).unwrap();
    let claims = verify_token(&config, &token).unwrap();

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.token_type, "refresh");
}

#[test]
fn jwt_verify_with_wrong_secret_fails() {
    let config = test_jwt_config();
    let token = issue_access_token(&config, Uuid::new_v4(), None, None).unwrap();

    let bad_config = JwtConfig::new("wrong-secret".to_string());
    let result = verify_token(&bad_config, &token);
    assert!(result.is_err(), "wrong secret should fail verification");
}

#[test]
fn jwt_verify_garbage_token_fails() {
    let config = test_jwt_config();
    let result = verify_token(&config, "not.a.jwt");
    assert!(result.is_err(), "garbage token should fail");
}

#[test]
fn jwt_expired_token_fails() {
    let config = JwtConfig {
        secret: "test-secret".to_string(),
        access_ttl: Duration::seconds(-120),
        refresh_ttl: Duration::days(7),
    };

    let token = issue_access_token(&config, Uuid::new_v4(), None, None).unwrap();
    let result = verify_token(&config, &token);
    assert!(result.is_err(), "expired token should fail verification");
}

#[tokio::test]
async fn service_signup_creates_user_and_returns_tokens() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("signup-{}@kahf.test", Uuid::new_v4());

    let resp = service::signup(db.pool(), &config, &email, "strong-password-123", "Test User")
        .await
        .unwrap();

    assert_eq!(resp.email, email);
    assert_eq!(resp.name, "Test User");
    assert!(!resp.access_token.is_empty());
    assert!(!resp.refresh_token.is_empty());

    let access_claims = verify_token(&config, &resp.access_token).unwrap();
    assert_eq!(access_claims.sub, resp.user_id);
    assert_eq!(access_claims.token_type, "access");

    let refresh_claims = verify_token(&config, &resp.refresh_token).unwrap();
    assert_eq!(refresh_claims.token_type, "refresh");
}

#[tokio::test]
async fn service_signup_duplicate_email_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("dup-signup-{}@kahf.test", Uuid::new_v4());

    service::signup(db.pool(), &config, &email, "pass1", "User 1").await.unwrap();

    let result = service::signup(db.pool(), &config, &email, "pass2", "User 2").await;
    assert!(result.is_err(), "duplicate email signup should fail");
}

#[tokio::test]
async fn service_login_success() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("login-{}@kahf.test", Uuid::new_v4());

    service::signup(db.pool(), &config, &email, "my-password", "Login User").await.unwrap();

    let resp = service::login(db.pool(), &config, &email, "my-password").await.unwrap();
    assert_eq!(resp.email, email);
    assert!(!resp.access_token.is_empty());
}

#[tokio::test]
async fn service_login_wrong_password_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("badpass-{}@kahf.test", Uuid::new_v4());

    service::signup(db.pool(), &config, &email, "correct-password", "User").await.unwrap();

    let result = service::login(db.pool(), &config, &email, "wrong-password").await;
    assert!(result.is_err(), "wrong password should fail login");
}

#[tokio::test]
async fn service_login_nonexistent_email_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();

    let result = service::login(db.pool(), &config, "nobody@kahf.test", "whatever").await;
    assert!(result.is_err(), "nonexistent email should fail login");
}

#[tokio::test]
async fn service_refresh_issues_new_access_token() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("refresh-{}@kahf.test", Uuid::new_v4());

    let signup_resp = service::signup(db.pool(), &config, &email, "pass", "Refresh User")
        .await.unwrap();

    let new_access = service::refresh(db.pool(), &config, &signup_resp.refresh_token)
        .await.unwrap();

    let claims = verify_token(&config, &new_access).unwrap();
    assert_eq!(claims.sub, signup_resp.user_id);
    assert_eq!(claims.token_type, "access");
}

#[tokio::test]
async fn service_refresh_with_access_token_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("ref-bad-{}@kahf.test", Uuid::new_v4());

    let resp = service::signup(db.pool(), &config, &email, "pass", "User").await.unwrap();

    let result = service::refresh(db.pool(), &config, &resp.access_token).await;
    assert!(result.is_err(), "using access token as refresh should fail");
}

#[tokio::test]
async fn service_refresh_with_invalid_token_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();

    let result = service::refresh(db.pool(), &config, "garbage.token.here").await;
    assert!(result.is_err(), "invalid refresh token should fail");
}
