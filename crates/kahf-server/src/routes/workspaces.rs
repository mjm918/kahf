//! Workspace management endpoints.
//!
//! ## POST /api/workspaces
//!
//! Creates a new workspace. The authenticated user becomes the owner.
//! Automatically assigns the `owner` RBAC role. Body: `{ name, slug }`.
//!
//! ## GET /api/workspaces
//!
//! Lists all workspaces the authenticated user belongs to.
//!
//! ## GET /api/workspaces/:id
//!
//! Returns a single workspace by ID.
//!
//! ## POST /api/workspaces/:id/members
//!
//! Adds a member to a workspace and assigns the specified RBAC role.
//! Requires `member:create` permission. Body: `{ user_id, role }`.
//!
//! ## DELETE /api/workspaces/:id/members/:user_id
//!
//! Removes a member from a workspace and revokes all RBAC roles.
//! Requires `member:delete` permission.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::Router;
use kahf_auth::AuthUser;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/workspaces", post(create_workspace).get(list_workspaces))
        .route("/api/workspaces/{id}", get(get_workspace))
        .route("/api/workspaces/{id}/members", post(add_member))
        .route("/api/workspaces/{id}/members/{user_id}", delete(remove_member))
}

#[derive(Deserialize)]
struct CreateWorkspaceRequest {
    name: String,
    slug: String,
}

async fn create_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::Json(body): axum::Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let ws = kahf_db::workspace_repo::create_workspace(
        state.pool(),
        &body.name,
        &body.slug,
        auth.claims.sub,
    )
    .await?;

    let ws_id = ws.id;
    kahf_rbac::assign_role(&state.rbac, auth.claims.sub, "owner", ws_id).await?;

    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(ws)?)))
}

async fn list_workspaces(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let workspaces = kahf_db::workspace_repo::list_user_workspaces(
        state.pool(),
        auth.claims.sub,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(workspaces)?))
}

async fn get_workspace(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ws = kahf_db::workspace_repo::get_workspace(state.pool(), id)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("workspace", id.to_string()))?;

    Ok(axum::Json(serde_json::to_value(ws)?))
}

#[derive(Deserialize)]
struct AddMemberRequest {
    user_id: Uuid,
    role: String,
}

async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    axum::Json(body): axum::Json<AddMemberRequest>,
) -> Result<StatusCode, AppError> {
    state.rbac.require(auth.claims.sub, id, "member", "create").await?;

    kahf_db::workspace_repo::add_member(
        state.pool(),
        id,
        body.user_id,
        &body.role,
    )
    .await?;

    kahf_rbac::assign_role(&state.rbac, body.user_id, &body.role, id).await?;

    Ok(StatusCode::CREATED)
}

async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    state.rbac.require(auth.claims.sub, id, "member", "delete").await?;

    kahf_db::workspace_repo::remove_member(state.pool(), id, user_id).await?;
    kahf_rbac::remove_all_roles(&state.rbac, user_id, id).await?;

    Ok(StatusCode::NO_CONTENT)
}
