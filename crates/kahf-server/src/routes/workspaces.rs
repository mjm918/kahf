//! Workspace management endpoints.
//!
//! All mutating operations emit audit log events for compliance tracking.
//!
//! ## POST /api/workspaces
//!
//! Creates a new workspace. The authenticated user becomes the owner.
//! Automatically assigns the `owner` RBAC role. Body: `{ name, slug,
//! color? }`. The `color` field defaults to Azure blue `#0078D4`.
//!
//! ## GET /api/workspaces
//!
//! Lists all workspaces the authenticated user belongs to.
//!
//! ## GET /api/workspaces/:id
//!
//! Returns a single workspace by ID.
//!
//! ## PATCH /api/workspaces/:id
//!
//! Updates a workspace's name and color. Requires `workspace:update`
//! permission. Body: `{ name?, color? }`.
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
//!
//! ## GET /api/workspaces/onboarding-status
//!
//! Returns whether the authenticated user needs onboarding (has no
//! workspaces). Used by frontend to redirect new users to workspace
//! creation flow.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::Router;
use kahf_auth::AuthUser;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::audit::{self, RequestContext};
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/workspaces", post(create_workspace).get(list_workspaces))
        .route("/api/workspaces/onboarding-status", get(onboarding_status))
        .route("/api/workspaces/{id}", get(get_workspace).patch(update_workspace))
        .route("/api/workspaces/{id}/members", post(add_member))
        .route("/api/workspaces/{id}/members/{user_id}", delete(remove_member))
}

#[derive(Deserialize)]
struct CreateWorkspaceRequest {
    name: String,
    slug: String,
    color: Option<String>,
}

async fn create_workspace(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    axum::Json(body): axum::Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let color = body.color.as_deref().unwrap_or("#0078D4");

    let ws = kahf_db::workspace_repo::create_workspace(
        state.pool(),
        &body.name,
        &body.slug,
        color,
        auth.claims.sub,
    )
    .await?;

    let ws_id = ws.id;
    kahf_rbac::assign_role(&state.rbac, auth.claims.sub, "owner", ws_id).await?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "workspace.create", Some(format!("workspace:{ws_id}")),
        "success", Some(serde_json::json!({"name": body.name, "slug": body.slug, "color": color})),
    ).await;

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
struct UpdateWorkspaceRequest {
    name: Option<String>,
    color: Option<String>,
}

async fn update_workspace(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    axum::Json(body): axum::Json<UpdateWorkspaceRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let existing = kahf_db::workspace_repo::get_workspace(state.pool(), id)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("workspace", id.to_string()))?;

    let name = body.name.as_deref().unwrap_or(&existing.name);
    let color = body.color.as_deref().unwrap_or(&existing.color);

    let ws = kahf_db::workspace_repo::update_workspace(state.pool(), id, name, color)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("workspace", id.to_string()))?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "workspace.update", Some(format!("workspace:{id}")),
        "success", Some(serde_json::json!({"name": name, "color": color})),
    ).await;

    Ok(axum::Json(serde_json::to_value(ws)?))
}

async fn onboarding_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let has_workspaces = kahf_db::workspace_repo::user_has_workspaces(
        state.pool(),
        auth.claims.sub,
    )
    .await?;

    Ok(axum::Json(serde_json::json!({
        "needs_onboarding": !has_workspaces,
    })))
}

#[derive(Deserialize)]
struct AddMemberRequest {
    user_id: Uuid,
    role: String,
}

async fn add_member(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
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

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "workspace.add_member", Some(format!("workspace:{id}")),
        "success", Some(serde_json::json!({
            "member_id": body.user_id,
            "role": body.role,
        })),
    ).await;

    Ok(StatusCode::CREATED)
}

async fn remove_member(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    state.rbac.require(auth.claims.sub, id, "member", "delete").await?;

    kahf_db::workspace_repo::remove_member(state.pool(), id, user_id).await?;
    kahf_rbac::remove_all_roles(&state.rbac, user_id, id).await?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "workspace.remove_member", Some(format!("workspace:{id}")),
        "success", Some(serde_json::json!({"removed_user_id": user_id})),
    ).await;

    Ok(StatusCode::NO_CONTENT)
}
