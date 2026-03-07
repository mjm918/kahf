//! Role-based access control using casbin-rs with PostgreSQL storage.
//!
//! Provides workspace-scoped RBAC with four roles: owner, admin,
//! member, and guest. Policies are stored in PostgreSQL via
//! sqlx-adapter and loaded into an in-memory casbin enforcer.
//!
//! ## RbacEnforcer
//!
//! Thread-safe enforcer wrapper. Use `check()` for boolean queries,
//! `require()` for guard-style checks that return `KahfError::Forbidden`.
//!
//! ## Policy helpers
//!
//! `assign_role`, `remove_role`, `remove_all_roles` manage user-role
//! assignments within workspaces.

pub mod enforcer;
pub mod model;
pub mod policy;

pub use enforcer::RbacEnforcer;
pub use policy::{assign_role, remove_all_roles, remove_role};
