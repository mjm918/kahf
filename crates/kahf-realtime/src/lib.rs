//! KahfLane real-time layer.
//!
//! Provides WebSocket-based real-time communication for the KahfLane platform.
//! Handles live entity updates, CRDT document collaboration, presence
//! tracking, and chat message routing.
//!
//! ## Modules
//!
//! - `event_bus` — `BroadcastEventBus`: in-process event bus via
//!   `tokio::sync::broadcast`, implements the `EventBus` trait
//! - `hub` — `Hub`: actor-pattern connection manager that routes
//!   messages between WebSocket clients, manages workspace and
//!   document rooms, and coordinates CRDT sync
//! - `connection` — `handle_connection`: drives a single WebSocket
//!   connection lifecycle (read/write split, hub integration)
//! - `protocol` — `WsMessage`: JSON-tagged enum covering all
//!   WebSocket message types (entity, CRDT, presence, chat)
//! - `crdt` — `CrdtManager`: yrs document store with lazy loading
//!   from PostgreSQL and persistence on update
//! - `presence` — `PresenceTracker`: per-workspace online user tracking

pub mod connection;
pub mod crdt;
pub mod event_bus;
pub mod hub;
pub mod presence;
pub mod protocol;

pub use connection::handle_connection;
pub use event_bus::BroadcastEventBus;
pub use hub::Hub;
pub use protocol::WsMessage;
