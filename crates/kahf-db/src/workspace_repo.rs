//! Workspace and workspace membership repository.
//!
//! ## WorkspaceRow
//!
//! Database row struct for the `workspaces` table. Includes a `color`
//! hex string used for UI theming of the workspace.
//!
//! ## WorkspaceMemberRow
//!
//! Database row struct for the `workspace_members` join table.
//! Includes `workspace_id`, `user_id`, `role`, and `joined_at`.
//!
//! ## create_workspace
//!
//! Creates a workspace and automatically adds the creator as `owner`
//! in `workspace_members` within a single transaction.
//!
//! ## get_workspace
//!
//! Fetches a workspace by ID.
//!
//! ## get_workspace_by_slug
//!
//! Fetches a workspace by unique slug.
//!
//! ## list_user_workspaces
//!
//! Returns all workspaces a user belongs to via `workspace_members`.
//!
//! ## update_workspace
//!
//! Updates a workspace's name and color.
//!
//! ## add_member
//!
//! Adds a member to a workspace. Uses `ON CONFLICT` to update role
//! if the member already exists.
//!
//! ## remove_member
//!
//! Removes a member from a workspace.
//!
//! ## list_members
//!
//! Returns all members of a workspace with user details (email,
//! first_name, last_name, avatar_url) joined from the users table.
//!
//! ## update_member_role
//!
//! Updates a member's role within a workspace.
//!
//! ## user_has_workspaces
//!
//! Returns whether a user belongs to at least one workspace.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub color: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceMemberRow {
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

pub async fn create_workspace(
    pool: &PgPool,
    name: &str,
    slug: &str,
    color: &str,
    created_by: Uuid,
) -> eyre::Result<WorkspaceRow> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, WorkspaceRow>(
        "INSERT INTO workspaces (name, slug, color, created_by) VALUES ($1, $2, $3, $4)
         RETURNING id, name, slug, color, created_by, created_at"
    )
    .bind(name)
    .bind(slug)
    .bind(color)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES ($1, $2, 'owner')"
    )
    .bind(row.id)
    .bind(created_by)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

pub async fn get_workspace(pool: &PgPool, id: Uuid) -> eyre::Result<Option<WorkspaceRow>> {
    let row = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT id, name, slug, color, created_by, created_at FROM workspaces WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_workspace_by_slug(pool: &PgPool, slug: &str) -> eyre::Result<Option<WorkspaceRow>> {
    let row = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT id, name, slug, color, created_by, created_at FROM workspaces WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_user_workspaces(pool: &PgPool, user_id: Uuid) -> eyre::Result<Vec<WorkspaceRow>> {
    let rows = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT w.id, w.name, w.slug, w.color, w.created_by, w.created_at
         FROM workspaces w
         INNER JOIN workspace_members wm ON w.id = wm.workspace_id
         WHERE wm.user_id = $1
         ORDER BY w.created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn update_workspace(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    color: &str,
) -> eyre::Result<Option<WorkspaceRow>> {
    let row = sqlx::query_as::<_, WorkspaceRow>(
        "UPDATE workspaces SET name = $2, color = $3
         WHERE id = $1
         RETURNING id, name, slug, color, created_by, created_at"
    )
    .bind(id)
    .bind(name)
    .bind(color)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn user_has_workspaces(pool: &PgPool, user_id: Uuid) -> eyre::Result<bool> {
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM workspace_members WHERE user_id = $1)"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn add_member(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> eyre::Result<()> {
    sqlx::query(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES ($1, $2, $3)
         ON CONFLICT (workspace_id, user_id) DO UPDATE SET role = EXCLUDED.role"
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceMemberDetail {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

pub async fn list_members(
    pool: &PgPool,
    workspace_id: Uuid,
) -> eyre::Result<Vec<WorkspaceMemberDetail>> {
    let rows = sqlx::query_as::<_, WorkspaceMemberDetail>(
        "SELECT wm.user_id, u.email, u.first_name, u.last_name, u.avatar_url,
                wm.role, wm.joined_at
         FROM workspace_members wm
         INNER JOIN users u ON u.id = wm.user_id
         WHERE wm.workspace_id = $1
         ORDER BY wm.joined_at ASC"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn update_member_role(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE workspace_members SET role = $3
         WHERE workspace_id = $1 AND user_id = $2"
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_member(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM workspace_members WHERE workspace_id = $1 AND user_id = $2")
        .bind(workspace_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}
