//! Casbin enforcer initialization and management.
//!
//! ## RbacEnforcer
//!
//! Thread-safe wrapper around `casbin::Enforcer` stored behind
//! `Arc<RwLock>`. Provides `check` for permission queries, `require`
//! for guard-style enforcement that returns `KahfError::Forbidden`,
//! and `write` for policy mutations.
//!
//! ## RbacEnforcer::new
//!
//! Builds an enforcer from the embedded RBAC model and a PostgreSQL-
//! backed sqlx-adapter. Loads default role policies if the policy
//! table is empty. Takes the database URL string directly since
//! sqlx-adapter creates its own connection pool.

use std::sync::Arc;

use casbin::prelude::*;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::model::{DEFAULT_POLICIES, RBAC_MODEL};

#[derive(Clone)]
pub struct RbacEnforcer {
    inner: Arc<RwLock<Enforcer>>,
}

impl RbacEnforcer {
    pub async fn new(database_url: &str) -> eyre::Result<Self> {
        let enforcer = create_enforcer(database_url).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(enforcer)),
        })
    }

    pub async fn check(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
        resource: &str,
        action: &str,
    ) -> eyre::Result<bool> {
        let enforcer = self.inner.read().await;
        let result = enforcer.enforce((
            user_id.to_string().as_str(),
            workspace_id.to_string().as_str(),
            resource,
            action,
        ))?;
        Ok(result)
    }

    pub async fn require(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
        resource: &str,
        action: &str,
    ) -> eyre::Result<()> {
        if !self.check(user_id, workspace_id, resource, action).await? {
            return Err(kahf_core::KahfError::forbidden());
        }
        Ok(())
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, Enforcer> {
        self.inner.write().await
    }
}

async fn create_enforcer(database_url: &str) -> eyre::Result<Enforcer> {
    let m = DefaultModel::from_str(RBAC_MODEL).await
        .map_err(|e| eyre::eyre!("failed to parse rbac model: {e}"))?;

    let a = sqlx_adapter::SqlxAdapter::new(database_url, 8).await
        .map_err(|e| eyre::eyre!("failed to create sqlx adapter: {e}"))?;

    let mut enforcer = Enforcer::new(m, a).await
        .map_err(|e| eyre::eyre!("failed to create enforcer: {e}"))?;

    let existing = enforcer.get_policy();
    if existing.is_empty() {
        tracing::info!("loading default RBAC policies");
        for (sub, dom, obj, act) in DEFAULT_POLICIES {
            enforcer
                .add_policy(vec![
                    sub.to_string(),
                    dom.to_string(),
                    obj.to_string(),
                    act.to_string(),
                ])
                .await
                .map_err(|e| eyre::eyre!("failed to add default policy: {e}"))?;
        }
    }

    Ok(enforcer)
}
