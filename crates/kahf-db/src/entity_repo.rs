//! EntityRepository trait implementation backed by the `entities` table.
//!
//! ## EntityRepository for DbPool
//!
//! Implements the `EntityRepository` trait from `kahf-core`.
//!
//! ## get
//!
//! Fetches a single entity by ID. Returns `None` if not found.
//!
//! ## list
//!
//! Lists entities in a workspace filtered by type, respecting pagination
//! (offset, limit) and sort order. Excludes soft-deleted entities.
//!
//! ## upsert
//!
//! Inserts or updates an entity using `ON CONFLICT` upsert on the
//! primary key. Updates data, updated_at, and deleted fields.
//!
//! ## soft_delete
//!
//! Sets `deleted = true` and `updated_at = now()` on the entity.
//!
//! ## row_to_entity
//!
//! Maps a `sqlx::postgres::PgRow` to a `kahf_core::entity::Entity`.

use sqlx::Row;

use kahf_core::entity::{Entity, EntityType};
use kahf_core::id::{EntityId, UserId, WorkspaceId};
use kahf_core::pagination::Pagination;
use kahf_core::traits::EntityRepository;

use crate::event_store::parse_entity_type;
use crate::pool::DbPool;

fn row_to_entity(row: sqlx::postgres::PgRow) -> eyre::Result<Entity> {
    Ok(Entity {
        id: EntityId::from_uuid(row.try_get("id")?),
        workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id")?),
        entity_type: parse_entity_type(row.try_get("type")?),
        data: row.try_get("data")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        created_by: UserId::from_uuid(row.try_get("created_by")?),
        deleted: row.try_get("deleted")?,
    })
}

impl EntityRepository for DbPool {
    async fn get(
        &self,
        id: EntityId,
    ) -> kahf_core::Result<Option<Entity>> {
        let row = sqlx::query(
            "SELECT id, workspace_id, type, data, created_at, updated_at, created_by, deleted
             FROM entities
             WHERE id = $1"
        )
        .bind(id.0)
        .fetch_optional(self.pool())
        .await?;

        row.map(row_to_entity).transpose()
    }

    async fn list(
        &self,
        workspace_id: WorkspaceId,
        entity_type: EntityType,
        pagination: Pagination,
    ) -> kahf_core::Result<Vec<Entity>> {
        let sort_column = match pagination.sort_by.as_deref() {
            Some("created_at") => "created_at",
            Some("type") => "type",
            _ => "updated_at",
        };
        let sort_dir = match pagination.sort_order {
            kahf_core::pagination::SortOrder::Asc => "ASC",
            kahf_core::pagination::SortOrder::Desc => "DESC",
        };

        let query_str = format!(
            "SELECT id, workspace_id, type, data, created_at, updated_at, created_by, deleted
             FROM entities
             WHERE workspace_id = $1 AND type = $2 AND NOT deleted
             ORDER BY {sort_column} {sort_dir}
             LIMIT $3 OFFSET $4"
        );

        let rows = sqlx::query(&query_str)
            .bind(workspace_id.0)
            .bind(entity_type.to_string())
            .bind(pagination.limit as i64)
            .bind(pagination.offset as i64)
            .fetch_all(self.pool())
            .await?;

        rows.into_iter().map(row_to_entity).collect()
    }

    async fn upsert(
        &self,
        entity: &Entity,
    ) -> kahf_core::Result<()> {
        sqlx::query(
            "INSERT INTO entities (id, workspace_id, type, data, created_at, updated_at, created_by, deleted)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                data = EXCLUDED.data,
                updated_at = EXCLUDED.updated_at,
                deleted = EXCLUDED.deleted"
        )
        .bind(entity.id.0)
        .bind(entity.workspace_id.0)
        .bind(entity.entity_type.to_string())
        .bind(&entity.data)
        .bind(entity.created_at)
        .bind(entity.updated_at)
        .bind(entity.created_by.0)
        .bind(entity.deleted)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn soft_delete(
        &self,
        id: EntityId,
    ) -> kahf_core::Result<()> {
        sqlx::query(
            "UPDATE entities SET deleted = true, updated_at = now() WHERE id = $1"
        )
        .bind(id.0)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
