//! Pagination and sorting primitives for list queries.
//!
//! ## SortOrder
//!
//! Enum with `Asc` (default) and `Desc` variants.
//!
//! ## Pagination
//!
//! Query parameters for list endpoints: `offset` (default 0),
//! `limit` (default 50), optional `sort_by` field name, and
//! `sort_order`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_offset")]
    pub offset: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: SortOrder,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            sort_by: None,
            sort_order: SortOrder::default(),
        }
    }
}

fn default_offset() -> u64 {
    0
}

fn default_limit() -> u64 {
    50
}
