//! Policy management helpers.
//!
//! ## assign_role
//!
//! Assigns a role to a user within a workspace by adding a grouping
//! policy `g, user:<uuid>, <role>, <workspace_uuid>`.
//!
//! ## remove_role
//!
//! Removes a user's role in a workspace.
//!
//! ## remove_all_roles
//!
//! Removes all roles for a user in a specific workspace.

use casbin::MgmtApi;
use uuid::Uuid;

use crate::enforcer::RbacEnforcer;

pub async fn assign_role(
    enforcer: &RbacEnforcer,
    user_id: Uuid,
    role: &str,
    workspace_id: Uuid,
) -> eyre::Result<()> {
    let mut e = enforcer.write().await;
    e.add_grouping_policy(vec![
        user_id.to_string(),
        role.to_string(),
        workspace_id.to_string(),
    ])
    .await
    .map_err(|e| eyre::eyre!("failed to assign role: {e}"))?;
    Ok(())
}

pub async fn remove_role(
    enforcer: &RbacEnforcer,
    user_id: Uuid,
    role: &str,
    workspace_id: Uuid,
) -> eyre::Result<()> {
    let mut e = enforcer.write().await;
    e.remove_grouping_policy(vec![
        user_id.to_string(),
        role.to_string(),
        workspace_id.to_string(),
    ])
    .await
    .map_err(|e| eyre::eyre!("failed to remove role: {e}"))?;
    Ok(())
}

pub async fn remove_all_roles(
    enforcer: &RbacEnforcer,
    user_id: Uuid,
    workspace_id: Uuid,
) -> eyre::Result<()> {
    let mut e = enforcer.write().await;
    e.remove_filtered_grouping_policy(
        0,
        vec![
            user_id.to_string(),
            String::new(),
            workspace_id.to_string(),
        ],
    )
    .await
    .map_err(|e| eyre::eyre!("failed to remove all roles: {e}"))?;
    Ok(())
}
