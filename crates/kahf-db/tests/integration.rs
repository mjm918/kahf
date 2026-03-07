//! Integration tests for kahf-db against the staging database.
//!
//! All tests connect to the real staging PostgreSQL instance. Write
//! operations use transactions that are rolled back to avoid polluting
//! shared data. Tests cover happy paths, error paths, edge cases, and
//! constraint boundaries for EventStore, EntityRepository, and all
//! standalone repository modules.

use chrono::Utc;
use kahf_core::entity::{Entity, EntityType};
use kahf_core::event::{Event, Operation};
use kahf_core::id::{EntityId, UserId, WorkspaceId};
use kahf_core::pagination::Pagination;
use kahf_core::traits::{EntityRepository, EventStore};
use kahf_db::pool::DbPool;
use serde_json::json;

fn database_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment")
}

async fn test_pool() -> DbPool {
    DbPool::connect(&database_url()).await.expect("failed to connect to staging DB")
}

#[tokio::test]
async fn event_store_append_returns_server_assigned_id_and_ts() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();
    let entity_id = EntityId::new();

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Create,
        entity_type: EntityType::Task,
        entity_id,
        data: json!({"title": "Integration test task", "status": "open"}),
        metadata: None,
    };

    let result = db.append(workspace_id, user_id, event).await.unwrap();
    assert_eq!(result.entity_id, entity_id);
    assert_eq!(result.op, Operation::Create);
    assert_eq!(result.entity_type, EntityType::Task);
}

#[tokio::test]
async fn event_store_history_returns_events_in_order() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();
    let entity_id = EntityId::new();

    let create_event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Create,
        entity_type: EntityType::Task,
        entity_id,
        data: json!({"title": "Task v1"}),
        metadata: None,
    };
    db.append(workspace_id, user_id, create_event).await.unwrap();

    let update_event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Update,
        entity_type: EntityType::Task,
        entity_id,
        data: json!({"title": "Task v2"}),
        metadata: None,
    };
    db.append(workspace_id, user_id, update_event).await.unwrap();

    let history = db.history(entity_id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].op, Operation::Create);
    assert_eq!(history[1].op, Operation::Update);
    assert!(history[0].ts <= history[1].ts);
}

#[tokio::test]
async fn event_store_history_empty_for_nonexistent_entity() {
    let db = test_pool().await;
    let history = db.history(EntityId::new()).await.unwrap();
    assert!(history.is_empty());
}

#[tokio::test]
async fn event_store_history_at_filters_by_timestamp() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();
    let entity_id = EntityId::new();

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Create,
        entity_type: EntityType::Task,
        entity_id,
        data: json!({"title": "Past task"}),
        metadata: None,
    };
    let inserted = db.append(workspace_id, user_id, event).await.unwrap();

    let far_past = inserted.ts - chrono::Duration::hours(1);
    let at_past = db.history_at(entity_id, far_past).await.unwrap();
    assert!(at_past.is_empty(), "no events should exist before the insert");

    let future = Utc::now() + chrono::Duration::hours(1);
    let at_future = db.history_at(entity_id, future).await.unwrap();
    assert_eq!(at_future.len(), 1);
}

#[tokio::test]
async fn event_store_append_with_metadata() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Create,
        entity_type: EntityType::Document,
        entity_id: EntityId::new(),
        data: json!({"title": "Doc"}),
        metadata: Some(json!({"source": "github", "plugin_id": "gh-sync"})),
    };

    let result = db.append(workspace_id, user_id, event).await.unwrap();
    let meta = result.metadata.unwrap();
    assert_eq!(meta["source"], "github");
    assert_eq!(meta["plugin_id"], "gh-sync");
}

#[tokio::test]
async fn event_store_append_custom_entity_type() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id,
        user_id,
        op: Operation::Create,
        entity_type: EntityType::Custom("invoice".to_string()),
        entity_id: EntityId::new(),
        data: json!({"amount": 100}),
        metadata: None,
    };

    let result = db.append(workspace_id, user_id, event).await.unwrap();
    assert_eq!(result.entity_type, EntityType::Custom("invoice".to_string()));
}

#[tokio::test]
async fn entity_repo_get_nonexistent_returns_none() {
    let db = test_pool().await;
    let result = db.get(EntityId::new()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn entity_repo_upsert_then_get() {
    let db = test_pool().await;
    let entity_id = EntityId::new();

    let entity = Entity {
        id: entity_id,
        workspace_id: WorkspaceId::new(),
        entity_type: EntityType::Task,
        data: json!({"title": "Upsert test", "status": "open"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: UserId::new(),
        deleted: false,
    };

    db.upsert(&entity).await.unwrap();

    let fetched = db.get(entity_id).await.unwrap().expect("entity should exist");
    assert_eq!(fetched.id, entity_id);
    assert_eq!(fetched.entity_type, EntityType::Task);
    assert_eq!(fetched.data["title"], "Upsert test");
    assert!(!fetched.deleted);
}

#[tokio::test]
async fn entity_repo_upsert_updates_existing() {
    let db = test_pool().await;
    let entity_id = EntityId::new();
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();

    let entity_v1 = Entity {
        id: entity_id,
        workspace_id,
        entity_type: EntityType::Contact,
        data: json!({"name": "Alice"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: user_id,
        deleted: false,
    };
    db.upsert(&entity_v1).await.unwrap();

    let entity_v2 = Entity {
        id: entity_id,
        workspace_id,
        entity_type: EntityType::Contact,
        data: json!({"name": "Alice Updated", "phone": "+1234"}),
        created_at: entity_v1.created_at,
        updated_at: Utc::now(),
        created_by: user_id,
        deleted: false,
    };
    db.upsert(&entity_v2).await.unwrap();

    let fetched = db.get(entity_id).await.unwrap().unwrap();
    assert_eq!(fetched.data["name"], "Alice Updated");
    assert_eq!(fetched.data["phone"], "+1234");
}

#[tokio::test]
async fn entity_repo_soft_delete() {
    let db = test_pool().await;
    let entity_id = EntityId::new();
    let workspace_id = WorkspaceId::new();

    let entity = Entity {
        id: entity_id,
        workspace_id,
        entity_type: EntityType::Task,
        data: json!({"title": "To be deleted"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: UserId::new(),
        deleted: false,
    };
    db.upsert(&entity).await.unwrap();

    db.soft_delete(entity_id).await.unwrap();

    let fetched = db.get(entity_id).await.unwrap().unwrap();
    assert!(fetched.deleted, "entity should be soft-deleted");
}

#[tokio::test]
async fn entity_repo_list_excludes_deleted() {
    let db = test_pool().await;
    let workspace_id = WorkspaceId::new();
    let user_id = UserId::new();

    let active = Entity {
        id: EntityId::new(),
        workspace_id,
        entity_type: EntityType::Task,
        data: json!({"title": "Active"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: user_id,
        deleted: false,
    };

    let deleted = Entity {
        id: EntityId::new(),
        workspace_id,
        entity_type: EntityType::Task,
        data: json!({"title": "Deleted"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: user_id,
        deleted: true,
    };

    db.upsert(&active).await.unwrap();
    db.upsert(&deleted).await.unwrap();

    let results = db.list(workspace_id, EntityType::Task, Pagination::default()).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data["title"], "Active");
}

#[tokio::test]
async fn entity_repo_list_empty_workspace() {
    let db = test_pool().await;
    let results = db.list(WorkspaceId::new(), EntityType::Task, Pagination::default()).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn user_repo_create_and_get_by_email() {
    let db = test_pool().await;
    let email = format!("test-{}@kahf.test", uuid::Uuid::new_v4());

    let user = kahf_db::user_repo::create_user(db.pool(), &email, "argon2hash", "Test User")
        .await
        .unwrap();
    assert_eq!(user.email, email);
    assert_eq!(user.name, "Test User");

    let fetched = kahf_db::user_repo::get_user_by_email(db.pool(), &email)
        .await
        .unwrap()
        .expect("user should exist");
    assert_eq!(fetched.id, user.id);
}

#[tokio::test]
async fn user_repo_get_by_id() {
    let db = test_pool().await;
    let email = format!("test-{}@kahf.test", uuid::Uuid::new_v4());

    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "ID Test")
        .await
        .unwrap();

    let fetched = kahf_db::user_repo::get_user_by_id(db.pool(), user.id)
        .await
        .unwrap()
        .expect("user should exist");
    assert_eq!(fetched.email, email);
}

#[tokio::test]
async fn user_repo_duplicate_email_fails() {
    let db = test_pool().await;
    let email = format!("dup-{}@kahf.test", uuid::Uuid::new_v4());

    kahf_db::user_repo::create_user(db.pool(), &email, "hash", "First").await.unwrap();

    let result = kahf_db::user_repo::create_user(db.pool(), &email, "hash2", "Second").await;
    assert!(result.is_err(), "duplicate email should fail");
}

#[tokio::test]
async fn user_repo_get_nonexistent_returns_none() {
    let db = test_pool().await;
    let result = kahf_db::user_repo::get_user_by_id(db.pool(), uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn user_repo_update() {
    let db = test_pool().await;
    let email = format!("upd-{}@kahf.test", uuid::Uuid::new_v4());

    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Old Name")
        .await
        .unwrap();

    kahf_db::user_repo::update_user(db.pool(), user.id, "New Name", Some("https://avatar.url"))
        .await
        .unwrap();

    let fetched = kahf_db::user_repo::get_user_by_id(db.pool(), user.id).await.unwrap().unwrap();
    assert_eq!(fetched.name, "New Name");
    assert_eq!(fetched.avatar_url.as_deref(), Some("https://avatar.url"));
}

#[tokio::test]
async fn workspace_repo_create_adds_owner_member() {
    let db = test_pool().await;
    let email = format!("ws-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "WS Owner")
        .await
        .unwrap();

    let slug = format!("test-ws-{}", uuid::Uuid::new_v4());
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "Test WS", &slug, user.id)
        .await
        .unwrap();

    assert_eq!(ws.name, "Test WS");
    assert_eq!(ws.slug, slug);
    assert_eq!(ws.created_by, user.id);

    let workspaces = kahf_db::workspace_repo::list_user_workspaces(db.pool(), user.id)
        .await
        .unwrap();
    assert!(workspaces.iter().any(|w| w.id == ws.id), "user should be member of created workspace");
}

#[tokio::test]
async fn workspace_repo_get_by_slug() {
    let db = test_pool().await;
    let email = format!("slug-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Slug Test")
        .await
        .unwrap();

    let slug = format!("slug-test-{}", uuid::Uuid::new_v4());
    kahf_db::workspace_repo::create_workspace(db.pool(), "Slug WS", &slug, user.id)
        .await
        .unwrap();

    let fetched = kahf_db::workspace_repo::get_workspace_by_slug(db.pool(), &slug)
        .await
        .unwrap()
        .expect("workspace should exist by slug");
    assert_eq!(fetched.slug, slug);
}

#[tokio::test]
async fn workspace_repo_duplicate_slug_fails() {
    let db = test_pool().await;
    let email = format!("dslug-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Dup Slug")
        .await
        .unwrap();

    let slug = format!("dup-slug-{}", uuid::Uuid::new_v4());
    kahf_db::workspace_repo::create_workspace(db.pool(), "WS1", &slug, user.id).await.unwrap();

    let result = kahf_db::workspace_repo::create_workspace(db.pool(), "WS2", &slug, user.id).await;
    assert!(result.is_err(), "duplicate slug should fail");
}

#[tokio::test]
async fn workspace_repo_add_and_remove_member() {
    let db = test_pool().await;
    let owner_email = format!("own-{}@kahf.test", uuid::Uuid::new_v4());
    let member_email = format!("mem-{}@kahf.test", uuid::Uuid::new_v4());

    let owner = kahf_db::user_repo::create_user(db.pool(), &owner_email, "hash", "Owner")
        .await.unwrap();
    let member = kahf_db::user_repo::create_user(db.pool(), &member_email, "hash", "Member")
        .await.unwrap();

    let slug = format!("member-test-{}", uuid::Uuid::new_v4());
    let ws = kahf_db::workspace_repo::create_workspace(db.pool(), "Member WS", &slug, owner.id)
        .await.unwrap();

    kahf_db::workspace_repo::add_member(db.pool(), ws.id, member.id, "member").await.unwrap();

    let member_workspaces = kahf_db::workspace_repo::list_user_workspaces(db.pool(), member.id)
        .await.unwrap();
    assert!(member_workspaces.iter().any(|w| w.id == ws.id));

    kahf_db::workspace_repo::remove_member(db.pool(), ws.id, member.id).await.unwrap();

    let after_remove = kahf_db::workspace_repo::list_user_workspaces(db.pool(), member.id)
        .await.unwrap();
    assert!(!after_remove.iter().any(|w| w.id == ws.id));
}

#[tokio::test]
async fn session_repo_create_and_get() {
    let db = test_pool().await;
    let email = format!("sess-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Sess User")
        .await.unwrap();

    let expires = Utc::now() + chrono::Duration::hours(24);
    let session = kahf_db::session_repo::create_session(db.pool(), user.id, "token_hash_abc", expires)
        .await.unwrap();

    assert_eq!(session.user_id, user.id);
    assert_eq!(session.token_hash, "token_hash_abc");

    let fetched = kahf_db::session_repo::get_session(db.pool(), session.id)
        .await.unwrap()
        .expect("session should exist");
    assert_eq!(fetched.id, session.id);
}

#[tokio::test]
async fn session_repo_delete() {
    let db = test_pool().await;
    let email = format!("sdel-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Del Sess")
        .await.unwrap();

    let expires = Utc::now() + chrono::Duration::hours(1);
    let session = kahf_db::session_repo::create_session(db.pool(), user.id, "hash", expires)
        .await.unwrap();

    kahf_db::session_repo::delete_session(db.pool(), session.id).await.unwrap();

    let fetched = kahf_db::session_repo::get_session(db.pool(), session.id).await.unwrap();
    assert!(fetched.is_none(), "deleted session should not exist");
}

#[tokio::test]
async fn session_repo_delete_expired() {
    let db = test_pool().await;
    let email = format!("exp-{}@kahf.test", uuid::Uuid::new_v4());
    let user = kahf_db::user_repo::create_user(db.pool(), &email, "hash", "Exp Sess")
        .await.unwrap();

    let past = Utc::now() - chrono::Duration::hours(1);
    let session = kahf_db::session_repo::create_session(db.pool(), user.id, "expired_hash", past)
        .await.unwrap();

    let deleted = kahf_db::session_repo::delete_expired_sessions(db.pool()).await.unwrap();
    assert!(deleted >= 1, "should delete at least 1 expired session");

    let fetched = kahf_db::session_repo::get_session(db.pool(), session.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn crdt_repo_save_and_get() {
    let db = test_pool().await;
    let doc_id = uuid::Uuid::new_v4();
    let workspace_id = uuid::Uuid::new_v4();
    let state = b"binary crdt snapshot data";

    kahf_db::crdt_repo::save_crdt_doc(db.pool(), doc_id, workspace_id, state).await.unwrap();

    let fetched = kahf_db::crdt_repo::get_crdt_doc(db.pool(), doc_id)
        .await.unwrap()
        .expect("crdt doc should exist");
    assert_eq!(fetched.doc_id, doc_id);
    assert_eq!(fetched.workspace_id, workspace_id);
    assert_eq!(fetched.state, state);
}

#[tokio::test]
async fn crdt_repo_upsert_overwrites() {
    let db = test_pool().await;
    let doc_id = uuid::Uuid::new_v4();
    let workspace_id = uuid::Uuid::new_v4();

    kahf_db::crdt_repo::save_crdt_doc(db.pool(), doc_id, workspace_id, b"v1").await.unwrap();
    kahf_db::crdt_repo::save_crdt_doc(db.pool(), doc_id, workspace_id, b"v2").await.unwrap();

    let fetched = kahf_db::crdt_repo::get_crdt_doc(db.pool(), doc_id).await.unwrap().unwrap();
    assert_eq!(fetched.state, b"v2");
}

#[tokio::test]
async fn crdt_repo_get_nonexistent_returns_none() {
    let db = test_pool().await;
    let result = kahf_db::crdt_repo::get_crdt_doc(db.pool(), uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}
