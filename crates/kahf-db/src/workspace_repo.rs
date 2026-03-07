//! Workspace and workspace membership repository.
//!
//! ## WorkspaceRow
//!
//! Database row struct for the `workspaces` table.
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
//! ## add_member
//!
//! Adds a member to a workspace. Uses `ON CONFLICT` to update role
//! if the member already exists.
//!
//! ## remove_member
//!
//! Removes a member from a workspace.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
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
    created_by: Uuid,
) -> eyre::Result<WorkspaceRow> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, WorkspaceRow>(
        "INSERT INTO workspaces (name, slug, created_by) VALUES ($1, $2, $3)
         RETURNING id, name, slug, created_by, created_at"
    )
    .bind(name)
    .bind(slug)
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
        "SELECT id, name, slug, created_by, created_at FROM workspaces WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_workspace_by_slug(pool: &PgPool, slug: &str) -> eyre::Result<Option<WorkspaceRow>> {
    let row = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT id, name, slug, created_by, created_at FROM workspaces WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_user_workspaces(pool: &PgPool, user_id: Uuid) -> eyre::Result<Vec<WorkspaceRow>> {
    let rows = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT w.id, w.name, w.slug, w.created_by, w.created_at
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
