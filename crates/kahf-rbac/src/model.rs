//! Casbin RBAC model definition with workspace domains.
//!
//! Embeds the PERM model as a compile-time constant. Uses domain-based
//! RBAC where workspaces act as domains. Each user is assigned a role
//! (owner, admin, member, guest) within a specific workspace.
//!
//! ## Request tuple
//!
//! `(sub, dom, obj, act)` where:
//!
//! - **sub**: user UUID string
//! - **dom**: workspace UUID string
//! - **obj**: resource category (`entity`, `member`, `workspace`, or `*`)
//! - **act**: action (`read`, `create`, `update`, `delete`, or `manage`)
//!
//! ## Wildcards
//!
//! Policy domain `*` matches any workspace. Policy object `*` matches
//! any resource. Action `manage` matches any action.

pub const RBAC_MODEL: &str = "\
[request_definition]\n\
r = sub, dom, obj, act\n\
\n\
[policy_definition]\n\
p = sub, dom, obj, act\n\
\n\
[role_definition]\n\
g = _, _, _\n\
\n\
[policy_effect]\n\
e = some(where (p.eft == allow))\n\
\n\
[matchers]\n\
m = g(r.sub, p.sub, r.dom) && (r.dom == p.dom || p.dom == \"*\") && (r.obj == p.obj || p.obj == \"*\") && (r.act == p.act || p.act == \"manage\")\n";

pub const DEFAULT_POLICIES: &[(&str, &str, &str, &str)] = &[
    ("owner", "*", "*", "manage"),
    ("admin", "*", "entity", "read"),
    ("admin", "*", "entity", "create"),
    ("admin", "*", "entity", "update"),
    ("admin", "*", "entity", "delete"),
    ("admin", "*", "member", "read"),
    ("admin", "*", "member", "create"),
    ("admin", "*", "member", "delete"),
    ("member", "*", "entity", "read"),
    ("member", "*", "entity", "create"),
    ("member", "*", "entity", "update"),
    ("guest", "*", "entity", "read"),
];
