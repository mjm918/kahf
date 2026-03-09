//! Invitation repository for the `invitations` table.
//!
//! ## InvitationRow
//!
//! Database row struct: `id`, `workspace_id`, `email`, `invited_by`,
//! `token`, `expires_at`, `accepted`, `created_at`.
//!
//! ## create_invitation
//!
//! Inserts a workspace-scoped invitation. Caller provides the workspace
//! ID, invitee email, inviter user ID, unique token, and expiration time.
//!
//! ## get_invitation_by_token
//!
//! Fetches a pending (non-expired, non-accepted) invitation by its
//! unique token. Returns `None` if no valid invitation exists.
//!
//! ## get_pending_by_email
//!
//! Fetches the most recent non-accepted invitation for the given email
//! within a workspace. Used by invite_user to cancel old invitations
//! before creating a fresh one.
//!
//! ## mark_invitation_accepted
//!
//! Sets `accepted = true` on the given invitation row.
//!
//! ## list_workspace_invitations
//!
//! Returns all non-accepted invitations for a workspace ordered by
//! creation date descending. Includes expired invitations so admins
//! can see and re-invite.
//!
//! ## cancel_invitation
//!
//! Deletes an invitation by ID.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InvitationRow {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub email: String,
    pub invited_by: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_invitation(
    pool: &PgPool,
    workspace_id: Uuid,
    email: &str,
    invited_by: Uuid,
    token: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<InvitationRow> {
    let row = sqlx::query_as::<_, InvitationRow>(
        "INSERT INTO invitations (workspace_id, email, invited_by, token, expires_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, workspace_id, email, invited_by, token, expires_at, accepted, created_at"
    )
    .bind(workspace_id)
    .bind(email)
    .bind(invited_by)
    .bind(token)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_invitation_by_token(pool: &PgPool, token: &str) -> eyre::Result<Option<InvitationRow>> {
    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT id, workspace_id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE token = $1 AND accepted = false AND expires_at > now()"
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_pending_by_email(pool: &PgPool, workspace_id: Uuid, email: &str) -> eyre::Result<Option<InvitationRow>> {
    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT id, workspace_id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE workspace_id = $1 AND email = $2 AND accepted = false
         ORDER BY created_at DESC
         LIMIT 1"
    )
    .bind(workspace_id)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn mark_invitation_accepted(pool: &PgPool, invitation_id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE invitations SET accepted = true WHERE id = $1")
        .bind(invitation_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_workspace_invitations(pool: &PgPool, workspace_id: Uuid) -> eyre::Result<Vec<InvitationRow>> {
    let rows = sqlx::query_as::<_, InvitationRow>(
        "SELECT id, workspace_id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE workspace_id = $1 AND accepted = false
         ORDER BY created_at DESC"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn cancel_invitation(pool: &PgPool, invitation_id: Uuid) -> eyre::Result<()> {
    sqlx::query("DELETE FROM invitations WHERE id = $1")
        .bind(invitation_id)
        .execute(pool)
        .await?;

    Ok(())
}
