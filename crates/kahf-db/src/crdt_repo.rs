//! CRDT document snapshot repository for the `crdt_docs` table.
//!
//! ## CrdtDocRow
//!
//! Database row struct: `doc_id`, `workspace_id`, `state` (binary yrs
//! snapshot), `updated_at`.
//!
//! ## save_crdt_doc
//!
//! Upserts a CRDT document snapshot. On conflict (existing doc),
//! overwrites the state and updated_at with the latest values.
//!
//! ## get_crdt_doc
//!
//! Fetches the latest snapshot for a document by ID.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CrdtDocRow {
    pub doc_id: Uuid,
    pub workspace_id: Uuid,
    pub state: Vec<u8>,
    pub updated_at: DateTime<Utc>,
}

pub async fn save_crdt_doc(
    pool: &PgPool,
    doc_id: Uuid,
    workspace_id: Uuid,
    state: &[u8],
) -> eyre::Result<()> {
    sqlx::query(
        "INSERT INTO crdt_docs (doc_id, workspace_id, state, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (doc_id) DO UPDATE SET state = EXCLUDED.state, updated_at = now()"
    )
    .bind(doc_id)
    .bind(workspace_id)
    .bind(state)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_crdt_doc(pool: &PgPool, doc_id: Uuid) -> eyre::Result<Option<CrdtDocRow>> {
    let row = sqlx::query_as::<_, CrdtDocRow>(
        "SELECT doc_id, workspace_id, state, updated_at FROM crdt_docs WHERE doc_id = $1"
    )
    .bind(doc_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
