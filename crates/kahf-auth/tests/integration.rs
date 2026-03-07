//! Integration tests for kahf-auth against the staging database.
//!
//! Tests cover password hashing, JWT token lifecycle, the OTP-based
//! signup/verify flow, login (requires verified email), refresh,
//! resend OTP, forgot/reset password, tenant-level invitations, tenant
//! creation during owner signup, first_name/last_name field handling,
//! and error paths including wrong passwords, expired tokens, duplicate
//! emails, invalid OTP codes, and unverified login attempts.

use chrono::Duration;
use kahf_email::{EmailSender, generate_otp, OTP_TTL_MINUTES};
use kahf_auth::jwt::{JwtConfig, issue_access_token, issue_refresh_token, verify_token};
use kahf_auth::password::{hash_password, verify_password};
use kahf_auth::service;
use kahf_db::pool::DbPool;
use uuid::Uuid;

struct NoopEmailSender;

impl EmailSender for NoopEmailSender {
    fn send_otp(&self, _to_email: &str, _otp_code: &str) -> eyre::Result<()> {
        Ok(())
    }

    fn send_password_reset_otp(&self, _to_email: &str, _otp_code: &str) -> eyre::Result<()> {
        Ok(())
    }

    fn send_invite(&self, _to_email: &str, _inviter_name: &str, _invite_token: &str) -> eyre::Result<()> {
        Ok(())
    }
}

fn database_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment")
}

fn test_jwt_config() -> JwtConfig {
    JwtConfig::new("test-secret-key-for-integration-tests".to_string())
}

async fn test_pool() -> DbPool {
    let db = DbPool::connect(&database_url()).await.expect("failed to connect to staging DB");
    db.migrate().await.expect("failed to run migrations");
    db
}

async fn create_verified_user(pool: &sqlx::PgPool, email: &str, password: &str, first_name: &str, last_name: &str) {
    let password_hash = hash_password(password).unwrap();
    let user = kahf_db::user_repo::create_user(pool, email, &password_hash, first_name, last_name)
        .await
        .unwrap();
    kahf_db::user_repo::mark_email_verified(pool, user.id).await.unwrap();
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

#[test]
fn otp_generate_is_six_digits() {
    for _ in 0..100 {
        let code = generate_otp();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }
}

#[tokio::test]
async fn service_signup_with_invite_returns_signup_response() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter-signup-{}@kahf.test", Uuid::new_v4());
    let signup_email = format!("signup-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Signup").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    service::invite_user(db.pool(), &mailer, inviter.id, &signup_email).await.unwrap();
    let invitation = kahf_db::invite_repo::get_pending_by_email(db.pool(), &signup_email).await.unwrap().unwrap();

    let resp = service::signup(
        db.pool(), &mailer, &signup_email, "strong-password-123", "Test", "User", None, Some(&invitation.token),
    )
    .await
    .unwrap();

    assert_eq!(resp.email, signup_email);
    assert!(!resp.message.is_empty());
    assert!(!resp.user_id.is_nil());
}

#[tokio::test]
async fn service_tenant_creation_via_repo() {
    let db = test_pool().await;
    let email = format!("tenant-owner-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Jane", "Doe")
        .await
        .unwrap();

    let tenant = kahf_db::tenant_repo::create_tenant(db.pool(), "Acme Inc", user.id)
        .await
        .unwrap();

    assert_eq!(tenant.company_name, "Acme Inc");
    assert_eq!(tenant.owner_id, user.id);

    let fetched = kahf_db::tenant_repo::get_tenant_by_owner(db.pool(), user.id)
        .await
        .unwrap()
        .expect("tenant should exist for owner");

    assert_eq!(fetched.id, tenant.id);
    assert_eq!(fetched.company_name, "Acme Inc");
}

#[tokio::test]
async fn service_signup_stores_first_and_last_name() {
    let db = test_pool().await;
    let email = format!("names-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("strong-password-123").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Alice", "Smith")
        .await
        .unwrap();

    assert_eq!(user.first_name, "Alice");
    assert_eq!(user.last_name, "Smith");
    assert_eq!(user.full_name(), "Alice Smith");
}

#[tokio::test]
async fn service_signup_duplicate_email_fails() {
    let db = test_pool().await;
    let email = format!("dup-signup-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("pass1").unwrap();
    kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "User", "One").await.unwrap();

    let result = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "User", "Two").await;
    assert!(result.is_err(), "duplicate email signup should fail");
}

#[tokio::test]
async fn service_verify_otp_success() {
    let db = test_pool().await;
    let jwt = test_jwt_config();
    let email = format!("verify-otp-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "OTP", "User")
        .await
        .unwrap();

    let otp_code = generate_otp();
    let expires_at = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, &otp_code, expires_at, "email_verification")
        .await
        .unwrap();

    let resp = service::verify_otp(db.pool(), &jwt, &email, &otp_code)
        .await
        .unwrap();

    assert_eq!(resp.email, email);
    assert_eq!(resp.user_id, user.id);
    assert_eq!(resp.first_name, "OTP");
    assert_eq!(resp.last_name, "User");
    assert!(!resp.access_token.is_empty());
    assert!(!resp.refresh_token.is_empty());

    let claims = verify_token(&jwt, &resp.access_token).unwrap();
    assert_eq!(claims.sub, user.id);
    assert_eq!(claims.token_type, "access");
}

#[tokio::test]
async fn service_verify_otp_wrong_code_fails() {
    let db = test_pool().await;
    let jwt = test_jwt_config();
    let email = format!("bad-otp-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Bad", "OTP")
        .await
        .unwrap();

    let expires_at = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, "123456", expires_at, "email_verification")
        .await
        .unwrap();

    let result = service::verify_otp(db.pool(), &jwt, &email, "999999").await;
    assert!(result.is_err(), "wrong OTP code should fail");
}

#[tokio::test]
async fn service_verify_otp_expired_code_fails() {
    let db = test_pool().await;
    let jwt = test_jwt_config();
    let email = format!("exp-otp-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Expired", "OTP")
        .await
        .unwrap();

    let expired = chrono::Utc::now() - Duration::minutes(1);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, "123456", expired, "email_verification")
        .await
        .unwrap();

    let result = service::verify_otp(db.pool(), &jwt, &email, "123456").await;
    assert!(result.is_err(), "expired OTP should fail");
}

#[tokio::test]
async fn service_verify_otp_already_verified_fails() {
    let db = test_pool().await;
    let jwt = test_jwt_config();
    let email = format!("already-verified-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "test-pass", "Already", "Verified").await;

    let result = service::verify_otp(db.pool(), &jwt, &email, "123456").await;
    assert!(result.is_err(), "already verified email should fail");
}

#[tokio::test]
async fn service_verify_otp_used_code_fails() {
    let db = test_pool().await;
    let jwt = test_jwt_config();
    let email = format!("used-otp-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Used", "OTP")
        .await
        .unwrap();

    let otp_code = generate_otp();
    let expires_at = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, &otp_code, expires_at, "email_verification")
        .await
        .unwrap();

    service::verify_otp(db.pool(), &jwt, &email, &otp_code).await.unwrap();

    let email2 = format!("used-otp2-{}@kahf.test", Uuid::new_v4());
    let user2 = kahf_db::user_repo::create_user(db.pool(), &email2, &password_hash, "Used", "OTP2")
        .await
        .unwrap();
    let expires_at2 = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    let otp2 = kahf_db::otp_repo::create_otp(db.pool(), user2.id, "654321", expires_at2, "email_verification")
        .await
        .unwrap();
    kahf_db::otp_repo::mark_otp_used(db.pool(), otp2.id).await.unwrap();

    let result = service::verify_otp(db.pool(), &jwt, &email2, "654321").await;
    assert!(result.is_err(), "used OTP code should fail");
}

#[tokio::test]
async fn service_login_success_verified_email() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("login-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "my-password", "Login", "User").await;

    let resp = service::login(db.pool(), &config, &email, "my-password").await.unwrap();
    assert_eq!(resp.email, email);
    assert_eq!(resp.first_name, "Login");
    assert_eq!(resp.last_name, "User");
    assert!(!resp.access_token.is_empty());
}

#[tokio::test]
async fn service_login_unverified_email_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("unverified-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("my-password").unwrap();
    kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Unverified", "User")
        .await
        .unwrap();

    let result = service::login(db.pool(), &config, &email, "my-password").await;
    assert!(result.is_err(), "unverified email should fail login");
}

#[tokio::test]
async fn service_login_wrong_password_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("badpass-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "correct-password", "Bad", "Pass").await;

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

    create_verified_user(db.pool(), &email, "pass", "Refresh", "User").await;

    let login_resp = service::login(db.pool(), &config, &email, "pass").await.unwrap();

    let new_access = service::refresh(db.pool(), &config, &login_resp.refresh_token)
        .await.unwrap();

    let claims = verify_token(&config, &new_access).unwrap();
    assert_eq!(claims.sub, login_resp.user_id);
    assert_eq!(claims.token_type, "access");
}

#[tokio::test]
async fn service_refresh_with_access_token_fails() {
    let db = test_pool().await;
    let config = test_jwt_config();
    let email = format!("ref-bad-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "pass", "Ref", "Bad").await;

    let resp = service::login(db.pool(), &config, &email, "pass").await.unwrap();

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

#[tokio::test]
async fn service_resend_otp_invalidates_old_codes() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let jwt = test_jwt_config();
    let email = format!("resend-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Resend", "User")
        .await
        .unwrap();

    let old_code = "111111";
    let expires_at = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, old_code, expires_at, "email_verification")
        .await
        .unwrap();

    service::resend_otp(db.pool(), &mailer, &email).await.unwrap();

    let result = service::verify_otp(db.pool(), &jwt, &email, old_code).await;
    assert!(result.is_err(), "old OTP should be invalidated after resend");
}

#[tokio::test]
async fn service_resend_otp_already_verified_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let email = format!("resend-verified-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "test-pass", "Verified", "Resend").await;

    let result = service::resend_otp(db.pool(), &mailer, &email).await;
    assert!(result.is_err(), "resend OTP for verified email should fail");
}

#[tokio::test]
async fn otp_repo_invalidate_all_user_otps() {
    let db = test_pool().await;
    let email = format!("inval-otp-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("test-pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Inval", "OTP")
        .await
        .unwrap();

    let expires_at = chrono::Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(db.pool(), user.id, "111111", expires_at, "email_verification").await.unwrap();
    kahf_db::otp_repo::create_otp(db.pool(), user.id, "222222", expires_at, "email_verification").await.unwrap();

    kahf_db::otp_repo::invalidate_user_otps(db.pool(), user.id, "email_verification").await.unwrap();

    let otp1 = kahf_db::otp_repo::get_valid_otp(db.pool(), user.id, "111111", "email_verification").await.unwrap();
    let otp2 = kahf_db::otp_repo::get_valid_otp(db.pool(), user.id, "222222", "email_verification").await.unwrap();
    assert!(otp1.is_none(), "all OTPs should be invalidated");
    assert!(otp2.is_none(), "all OTPs should be invalidated");
}

#[tokio::test]
async fn service_forgot_password_returns_success_for_existing_user() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let email = format!("forgot-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "old-password", "Forgot", "User").await;

    let resp = service::forgot_password(db.pool(), &mailer, &email).await.unwrap();
    assert!(!resp.message.is_empty());
}

#[tokio::test]
async fn service_forgot_password_returns_success_for_nonexistent_email() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;

    let resp = service::forgot_password(db.pool(), &mailer, "nobody@kahf.test").await.unwrap();
    assert!(!resp.message.is_empty());
}

#[tokio::test]
async fn service_forgot_password_ignores_unverified_user() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let email = format!("unverified-forgot-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("pass").unwrap();
    kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Unverified", "Forgot").await.unwrap();

    let resp = service::forgot_password(db.pool(), &mailer, &email).await.unwrap();
    assert!(!resp.message.is_empty());
}

#[tokio::test]
async fn service_reset_password_success() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let config = test_jwt_config();
    let email = format!("reset-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "old-password", "Reset", "User").await;

    service::forgot_password(db.pool(), &mailer, &email).await.unwrap();

    let user = kahf_db::user_repo::get_user_by_email(db.pool(), &email).await.unwrap().unwrap();
    let otp = sqlx::query_as::<_, kahf_db::otp_repo::OtpRow>(
        "SELECT id, user_id, code, purpose, expires_at, used, created_at FROM email_otps WHERE user_id = $1 AND purpose = 'password_reset' AND used = false ORDER BY created_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_one(db.pool())
    .await
    .unwrap();

    let resp = service::reset_password(db.pool(), &email, &otp.code, "new-password-123").await.unwrap();
    assert!(!resp.message.is_empty());

    let login_resp = service::login(db.pool(), &config, &email, "new-password-123").await.unwrap();
    assert_eq!(login_resp.email, email);

    let old_login = service::login(db.pool(), &config, &email, "old-password").await;
    assert!(old_login.is_err(), "old password should no longer work");
}

#[tokio::test]
async fn service_reset_password_wrong_code_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let email = format!("reset-bad-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &email, "old-password", "Reset", "Bad").await;

    service::forgot_password(db.pool(), &mailer, &email).await.unwrap();

    let result = service::reset_password(db.pool(), &email, "000000", "new-password").await;
    assert!(result.is_err(), "wrong reset code should fail");
}

#[tokio::test]
async fn service_reset_password_nonexistent_email_fails() {
    let db = test_pool().await;

    let result = service::reset_password(db.pool(), "nobody@kahf.test", "123456", "new-pass").await;
    assert!(result.is_err(), "nonexistent email should fail reset");
}

#[tokio::test]
async fn service_invite_user_success() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter-{}@kahf.test", Uuid::new_v4());
    let invitee_email = format!("invitee-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "User").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    let resp = service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await.unwrap();
    assert_eq!(resp.email, invitee_email);
    assert!(!resp.invitation_id.is_nil());
}

#[tokio::test]
async fn service_invite_existing_user_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter2-{}@kahf.test", Uuid::new_v4());
    let existing_email = format!("existing-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Two").await;
    create_verified_user(db.pool(), &existing_email, "pass", "Existing", "User").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    let result = service::invite_user(db.pool(), &mailer, inviter.id, &existing_email).await;
    assert!(result.is_err(), "inviting existing user should fail");
}

#[tokio::test]
async fn service_invite_duplicate_pending_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter3-{}@kahf.test", Uuid::new_v4());
    let invitee_email = format!("invitee3-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Three").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await.unwrap();
    let result = service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await;
    assert!(result.is_err(), "duplicate pending invitation should fail");
}

#[tokio::test]
async fn service_validate_invite_success() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter4-{}@kahf.test", Uuid::new_v4());
    let invitee_email = format!("invitee4-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Four").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await.unwrap();

    let invitation = kahf_db::invite_repo::get_pending_by_email(db.pool(), &invitee_email).await.unwrap().unwrap();
    let validation = service::validate_invite(db.pool(), &invitation.token).await.unwrap();
    assert_eq!(validation.email, invitee_email);
}

#[tokio::test]
async fn service_validate_invite_invalid_token_fails() {
    let db = test_pool().await;

    let result = service::validate_invite(db.pool(), "nonexistent-token").await;
    assert!(result.is_err(), "invalid invite token should fail");
}

#[tokio::test]
async fn service_signup_with_invite_token_success() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter5-{}@kahf.test", Uuid::new_v4());
    let invitee_email = format!("invitee5-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Five").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await.unwrap();
    let invitation = kahf_db::invite_repo::get_pending_by_email(db.pool(), &invitee_email).await.unwrap().unwrap();

    let resp = service::signup(
        db.pool(), &mailer, &invitee_email, "new-pass", "Invitee", "Five", None, Some(&invitation.token),
    ).await.unwrap();

    assert_eq!(resp.email, invitee_email);

    let accepted = kahf_db::invite_repo::get_invitation_by_token(db.pool(), &invitation.token).await.unwrap();
    assert!(accepted.is_none(), "accepted invitation should not be returned as pending");
}

#[tokio::test]
async fn service_signup_with_wrong_invite_token_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let email = format!("bad-invite-{}@kahf.test", Uuid::new_v4());

    let result = service::signup(db.pool(), &mailer, &email, "pass", "Bad", "Invite", None, Some("bad-token")).await;
    assert!(result.is_err(), "signup with invalid invite token should fail");
}

#[tokio::test]
async fn service_signup_with_mismatched_invite_email_fails() {
    let db = test_pool().await;
    let mailer = NoopEmailSender;
    let inviter_email = format!("inviter6-{}@kahf.test", Uuid::new_v4());
    let invitee_email = format!("invitee6-{}@kahf.test", Uuid::new_v4());
    let wrong_email = format!("wrong-{}@kahf.test", Uuid::new_v4());

    create_verified_user(db.pool(), &inviter_email, "pass", "Inviter", "Six").await;
    let inviter = kahf_db::user_repo::get_user_by_email(db.pool(), &inviter_email).await.unwrap().unwrap();

    service::invite_user(db.pool(), &mailer, inviter.id, &invitee_email).await.unwrap();
    let invitation = kahf_db::invite_repo::get_pending_by_email(db.pool(), &invitee_email).await.unwrap().unwrap();

    let result = service::signup(
        db.pool(), &mailer, &wrong_email, "pass", "Wrong", "User", None, Some(&invitation.token),
    ).await;
    assert!(result.is_err(), "signup with mismatched email should fail");
}

#[tokio::test]
async fn tenant_repo_get_by_id() {
    let db = test_pool().await;
    let email = format!("tenant-id-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "Tenant", "ID")
        .await
        .unwrap();

    let tenant = kahf_db::tenant_repo::create_tenant(db.pool(), "ID Corp", user.id)
        .await
        .unwrap();

    let fetched = kahf_db::tenant_repo::get_tenant_by_id(db.pool(), tenant.id)
        .await
        .unwrap()
        .expect("tenant should exist");

    assert_eq!(fetched.company_name, "ID Corp");
    assert_eq!(fetched.owner_id, user.id);
}

#[tokio::test]
async fn tenant_repo_get_nonexistent_returns_none() {
    let db = test_pool().await;

    let result = kahf_db::tenant_repo::get_tenant_by_id(db.pool(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn user_full_name_helper() {
    let db = test_pool().await;
    let email = format!("fullname-{}@kahf.test", Uuid::new_v4());

    let password_hash = hash_password("pass").unwrap();
    let user = kahf_db::user_repo::create_user(db.pool(), &email, &password_hash, "John", "Doe")
        .await
        .unwrap();

    assert_eq!(user.full_name(), "John Doe");
}
