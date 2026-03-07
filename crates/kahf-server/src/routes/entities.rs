//! Generic entity CRUD endpoints and time-travel history.
//!
//! All entity types (task, contact, document, etc.) share the same
//! REST interface. The entity type is a path parameter parsed from
//! the URL.
//!
//! ## GET /api/entities/:type
//!
//! Lists entities of a given type within the workspace specified in
//! the JWT claims. Supports pagination via query parameters.
//!
//! ## GET /api/entities/:type/:id
//!
//! Returns a single entity by ID.
//!
//! ## POST /api/entities/:type
//!
//! Creates a new entity. Appends a `Create` event to the tx_log and
//! upserts the materialized entity. Body is the JSON entity data.
//!
//! ## PATCH /api/entities/:type/:id
//!
//! Updates an entity with a partial JSON patch. Appends an `Update`
//! event and upserts the merged entity.
//!
//! ## DELETE /api/entities/:type/:id
//!
//! Soft-deletes an entity. Appends a `Delete` event and marks the
//! entity as deleted.
//!
//! ## GET /api/entities/:type/:id/history
//!
//! Returns the full event history for an entity from the tx_log.
//!
//! ## GET /api/entities/:type/:id/at
//!
//! Returns the entity's event history up to a given timestamp.
//! Query parameter: `ts` (RFC 3339 timestamp).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Utc};
use kahf_auth::AuthUser;
use kahf_core::traits::{EntityRepository, EventStore};
use kahf_core::{Entity, EntityId, EntityType, Event, Operation, Pagination, UserId, WorkspaceId};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/entities/{entity_type}", get(list_entities).post(create_entity))
        .route(
            "/api/entities/{entity_type}/{id}",
            get(get_entity).patch(update_entity).delete(delete_entity),
        )
        .route("/api/entities/{entity_type}/{id}/history", get(entity_history))
        .route("/api/entities/{entity_type}/{id}/at", get(entity_at))
}

fn parse_entity_type(s: &str) -> EntityType {
    kahf_db::event_store::parse_entity_type(s)
}

async fn list_entities(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(entity_type): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "read").await?;
    let et = parse_entity_type(&entity_type);
    let entities = state.db.list(ws_id, et, pagination).await?;
    Ok(axum::Json(serde_json::to_value(entities)?))
}

async fn get_entity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_entity_type, id)): Path<(String, Uuid)>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "read").await?;
    let entity_id = EntityId::from_uuid(id);
    let entity = state
        .db
        .get(entity_id)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("entity", id.to_string()))?;

    Ok(axum::Json(serde_json::to_value(entity)?))
}

async fn create_entity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(entity_type): Path<String>,
    axum::Json(data): axum::Json<Value>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "create").await?;
    let user_id = UserId::from_uuid(auth.claims.sub);
    let et = parse_entity_type(&entity_type);
    let et_str = et.to_string();
    let entity_id = EntityId::new();
    let now = Utc::now();

    let event = Event {
        id: EntityId::new(),
        ts: now,
        workspace_id: ws_id,
        user_id,
        op: Operation::Create,
        entity_type: et.clone(),
        entity_id,
        data: data.clone(),
        metadata: None,
    };

    state.db.append(ws_id, user_id, event).await?;

    let entity = Entity {
        id: entity_id,
        workspace_id: ws_id,
        entity_type: et,
        data,
        created_at: now,
        updated_at: now,
        created_by: user_id,
        deleted: false,
    };

    state.db.upsert(&entity).await?;

    state.hub.broadcast(
        ws_id.0,
        kahf_realtime::WsMessage::EntityCreated {
            entity_type: et_str,
            id: entity_id.0,
            data: entity.data.clone(),
        },
    );

    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(entity)?)))
}

async fn update_entity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_entity_type, id)): Path<(String, Uuid)>,
    axum::Json(patch_data): axum::Json<Value>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "update").await?;
    let user_id = UserId::from_uuid(auth.claims.sub);
    let entity_id = EntityId::from_uuid(id);

    let mut entity = state
        .db
        .get(entity_id)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("entity", id.to_string()))?;

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id: ws_id,
        user_id,
        op: Operation::Update,
        entity_type: entity.entity_type.clone(),
        entity_id,
        data: patch_data.clone(),
        metadata: None,
    };

    state.db.append(ws_id, user_id, event).await?;

    merge_json(&mut entity.data, &patch_data);
    entity.updated_at = Utc::now();
    state.db.upsert(&entity).await?;

    state.hub.broadcast(
        ws_id.0,
        kahf_realtime::WsMessage::EntityUpdated {
            entity_type: entity.entity_type.to_string(),
            id: entity_id.0,
            patch: patch_data,
        },
    );

    Ok(axum::Json(serde_json::to_value(entity)?))
}

async fn delete_entity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((entity_type, id)): Path<(String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "delete").await?;
    let user_id = UserId::from_uuid(auth.claims.sub);
    let entity_id = EntityId::from_uuid(id);
    let et = parse_entity_type(&entity_type);

    let event = Event {
        id: EntityId::new(),
        ts: Utc::now(),
        workspace_id: ws_id,
        user_id,
        op: Operation::Delete,
        entity_type: et.clone(),
        entity_id,
        data: serde_json::json!({}),
        metadata: None,
    };

    state.db.append(ws_id, user_id, event).await?;
    state.db.soft_delete(entity_id).await?;

    state.hub.broadcast(
        ws_id.0,
        kahf_realtime::WsMessage::EntityDeleted {
            entity_type: et.to_string(),
            id,
        },
    );

    Ok(StatusCode::NO_CONTENT)
}

async fn entity_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_entity_type, id)): Path<(String, Uuid)>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "read").await?;
    let entity_id = EntityId::from_uuid(id);
    let events = state.db.history(entity_id).await?;
    Ok(axum::Json(serde_json::to_value(events)?))
}

#[derive(Deserialize)]
struct AtQuery {
    ts: DateTime<Utc>,
}

async fn entity_at(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_entity_type, id)): Path<(String, Uuid)>,
    Query(query): Query<AtQuery>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws_id = workspace_id_from_claims(&auth)?;
    state.rbac.require(auth.claims.sub, ws_id.0, "entity", "read").await?;
    let entity_id = EntityId::from_uuid(id);
    let events = state.db.history_at(entity_id, query.ts).await?;
    Ok(axum::Json(serde_json::to_value(events)?))
}

fn workspace_id_from_claims(auth: &AuthUser) -> Result<WorkspaceId, AppError> {
    let ws_uuid = auth
        .claims
        .workspace_id
        .ok_or_else(|| kahf_core::KahfError::validation("workspace_id missing from token"))?;
    Ok(WorkspaceId::from_uuid(ws_uuid))
}

fn merge_json(base: &mut Value, patch: &Value) {
    if let (Some(base_obj), Some(patch_obj)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
}
