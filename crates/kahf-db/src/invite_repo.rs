//! Invitation repository for the `invitations` table.
//!
//! ## InvitationRow
//!
//! Database row struct: `id`, `email`, `invited_by`, `token`,
//! `expires_at`, `accepted`, `created_at`.
//!
//! ## create_invitation
//!
//! Inserts a new tenant-level invitation. Caller provides the invitee
//! email, inviter user ID, unique token, and expiration time.
//!
//! ## get_invitation_by_token
//!
//! Fetches a pending (non-expired, non-accepted) invitation by its
//! unique token. Returns `None` if no valid invitation exists.
//!
//! ## get_pending_by_email
//!
//! Fetches a pending invitation for the given email address.
//!
//! ## mark_invitation_accepted
//!
//! Sets `accepted = true` on the given invitation row.
//!
//! ## list_pending_invitations
//!
//! Returns all non-expired, non-accepted invitations ordered by
//! creation date descending.
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
    pub email: String,
    pub invited_by: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_invitation(
    pool: &PgPool,
    email: &str,
    invited_by: Uuid,
    token: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<InvitationRow> {
    let row = sqlx::query_as::<_, InvitationRow>(
        "INSERT INTO invitations (email, invited_by, token, expires_at)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, invited_by, token, expires_at, accepted, created_at"
    )
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
        "SELECT id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE token = $1 AND accepted = false AND expires_at > now()"
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_pending_by_email(pool: &PgPool, email: &str) -> eyre::Result<Option<InvitationRow>> {
    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE email = $1 AND accepted = false AND expires_at > now()
         ORDER BY created_at DESC
         LIMIT 1"
    )
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

pub async fn list_pending_invitations(pool: &PgPool) -> eyre::Result<Vec<InvitationRow>> {
    let rows = sqlx::query_as::<_, InvitationRow>(
        "SELECT id, email, invited_by, token, expires_at, accepted, created_at
         FROM invitations
         WHERE accepted = false AND expires_at > now()
         ORDER BY created_at DESC"
    )
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
