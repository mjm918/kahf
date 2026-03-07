//! Tenant repository for the `tenants` table.
//!
//! ## TenantRow
//!
//! Database row struct matching the `tenants` table columns: `id`,
//! `company_name`, `owner_id`, `created_at`.
//!
//! ## create_tenant
//!
//! Inserts a new tenant with the given company name and owner user ID.
//! Returns the created `TenantRow`.
//!
//! ## get_tenant_by_owner
//!
//! Fetches the tenant owned by a given user ID.
//!
//! ## get_tenant_by_id
//!
//! Fetches a tenant by its UUID primary key.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantRow {
    pub id: Uuid,
    pub company_name: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
}

pub async fn create_tenant(
    pool: &PgPool,
    company_name: &str,
    owner_id: Uuid,
) -> eyre::Result<TenantRow> {
    let row = sqlx::query_as::<_, TenantRow>(
        "INSERT INTO tenants (company_name, owner_id) VALUES ($1, $2)
         RETURNING id, company_name, owner_id, created_at"
    )
    .bind(company_name)
    .bind(owner_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_tenant_by_owner(pool: &PgPool, owner_id: Uuid) -> eyre::Result<Option<TenantRow>> {
    let row = sqlx::query_as::<_, TenantRow>(
        "SELECT id, company_name, owner_id, created_at FROM tenants WHERE owner_id = $1"
    )
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_tenant_by_id(pool: &PgPool, id: Uuid) -> eyre::Result<Option<TenantRow>> {
    let row = sqlx::query_as::<_, TenantRow>(
        "SELECT id, company_name, owner_id, created_at FROM tenants WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
