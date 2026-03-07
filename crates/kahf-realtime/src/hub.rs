//! WebSocket connection hub using the actor pattern.
//!
//! ## Hub
//!
//! Manages all active WebSocket connections, workspace rooms, document
//! rooms, CRDT state, and presence. Runs as a single tokio task that
//! processes commands sequentially from an unbounded mpsc channel.
//! All mutations to hub state are serialized through this channel,
//! eliminating the need for locks.
//!
//! ## HubCmd
//!
//! Commands sent to the hub task: `Connect`, `Disconnect`, `JoinDoc`,
//! `LeaveDoc`, `Incoming` (client message), and `Broadcast` (server-
//! initiated broadcast to a workspace).
//!
//! ## Hub::new
//!
//! Creates the hub and spawns the background actor task.
//! Returns a `Hub` handle that is `Clone + Send + Sync`.
//!
//! ## Hub::connect / disconnect / join_doc / leave_doc / incoming / broadcast
//!
//! Fire-and-forget methods that send commands to the hub task.

use std::collections::{HashMap, HashSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::crdt::CrdtManager;
use crate::presence::PresenceTracker;
use crate::protocol::WsMessage;

struct ConnectionInfo {
    user_id: Uuid,
    workspace_id: Uuid,
    tx: mpsc::UnboundedSender<String>,
}

pub enum HubCmd {
    Connect {
        conn_id: Uuid,
        user_id: Uuid,
        workspace_id: Uuid,
        tx: mpsc::UnboundedSender<String>,
    },
    Disconnect {
        conn_id: Uuid,
    },
    JoinDoc {
        conn_id: Uuid,
        doc_id: Uuid,
    },
    LeaveDoc {
        conn_id: Uuid,
        doc_id: Uuid,
    },
    Incoming {
        conn_id: Uuid,
        message: WsMessage,
    },
    Broadcast {
        workspace_id: Uuid,
        message: WsMessage,
    },
}

#[derive(Clone)]
pub struct Hub {
    cmd_tx: mpsc::UnboundedSender<HubCmd>,
}

impl Hub {
    pub fn new(pool: PgPool) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let inner = HubInner::new(pool);
        tokio::spawn(inner.run(cmd_rx));
        Self { cmd_tx }
    }

    pub fn connect(
        &self,
        conn_id: Uuid,
        user_id: Uuid,
        workspace_id: Uuid,
        tx: mpsc::UnboundedSender<String>,
    ) {
        let _ = self.cmd_tx.send(HubCmd::Connect {
            conn_id,
            user_id,
            workspace_id,
            tx,
        });
    }

    pub fn disconnect(&self, conn_id: Uuid) {
        let _ = self.cmd_tx.send(HubCmd::Disconnect { conn_id });
    }

    pub fn join_doc(&self, conn_id: Uuid, doc_id: Uuid) {
        let _ = self.cmd_tx.send(HubCmd::JoinDoc { conn_id, doc_id });
    }

    pub fn leave_doc(&self, conn_id: Uuid, doc_id: Uuid) {
        let _ = self.cmd_tx.send(HubCmd::LeaveDoc { conn_id, doc_id });
    }

    pub fn incoming(&self, conn_id: Uuid, message: WsMessage) {
        let _ = self.cmd_tx.send(HubCmd::Incoming { conn_id, message });
    }

    pub fn broadcast(&self, workspace_id: Uuid, message: WsMessage) {
        let _ = self.cmd_tx.send(HubCmd::Broadcast {
            workspace_id,
            message,
        });
    }
}

struct HubInner {
    connections: HashMap<Uuid, ConnectionInfo>,
    workspace_rooms: HashMap<Uuid, HashSet<Uuid>>,
    doc_rooms: HashMap<Uuid, HashSet<Uuid>>,
    conn_docs: HashMap<Uuid, HashSet<Uuid>>,
    crdt: CrdtManager,
    presence: PresenceTracker,
}

impl HubInner {
    fn new(pool: PgPool) -> Self {
        Self {
            connections: HashMap::new(),
            workspace_rooms: HashMap::new(),
            doc_rooms: HashMap::new(),
            conn_docs: HashMap::new(),
            crdt: CrdtManager::new(pool),
            presence: PresenceTracker::new(),
        }
    }

    async fn run(mut self, mut cmd_rx: mpsc::UnboundedReceiver<HubCmd>) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                HubCmd::Connect { conn_id, user_id, workspace_id, tx } => {
                    self.handle_connect(conn_id, user_id, workspace_id, tx);
                }
                HubCmd::Disconnect { conn_id } => {
                    self.handle_disconnect(conn_id);
                }
                HubCmd::JoinDoc { conn_id, doc_id } => {
                    self.handle_join_doc(conn_id, doc_id).await;
                }
                HubCmd::LeaveDoc { conn_id, doc_id } => {
                    self.handle_leave_doc(conn_id, doc_id);
                }
                HubCmd::Incoming { conn_id, message } => {
                    self.handle_incoming(conn_id, message).await;
                }
                HubCmd::Broadcast { workspace_id, message } => {
                    self.send_to_workspace(workspace_id, &message, None);
                }
            }
        }
    }

    fn handle_connect(
        &mut self,
        conn_id: Uuid,
        user_id: Uuid,
        workspace_id: Uuid,
        tx: mpsc::UnboundedSender<String>,
    ) {
        tracing::info!(conn_id = %conn_id, user_id = %user_id, "client connected");

        self.connections.insert(conn_id, ConnectionInfo { user_id, workspace_id, tx });

        self.workspace_rooms
            .entry(workspace_id)
            .or_default()
            .insert(conn_id);

        if self.presence.user_online(workspace_id, user_id) {
            let msg = WsMessage::PresenceUpdate {
                user: user_id,
                status: "online".into(),
            };
            self.send_to_workspace(workspace_id, &msg, Some(conn_id));
        }
    }

    fn handle_disconnect(&mut self, conn_id: Uuid) {
        let Some(info) = self.connections.remove(&conn_id) else {
            return;
        };

        tracing::info!(conn_id = %conn_id, user_id = %info.user_id, "client disconnected");

        if let Some(conns) = self.workspace_rooms.get_mut(&info.workspace_id) {
            conns.remove(&conn_id);
            if conns.is_empty() {
                self.workspace_rooms.remove(&info.workspace_id);
            }
        }

        if let Some(doc_ids) = self.conn_docs.remove(&conn_id) {
            for doc_id in doc_ids {
                if let Some(conns) = self.doc_rooms.get_mut(&doc_id) {
                    conns.remove(&conn_id);
                    if conns.is_empty() {
                        self.doc_rooms.remove(&doc_id);
                    }
                }
            }
        }

        let still_connected = self.workspace_rooms
            .get(&info.workspace_id)
            .map(|conns| {
                conns.iter().any(|c| {
                    self.connections.get(c).is_some_and(|ci| ci.user_id == info.user_id)
                })
            })
            .unwrap_or(false);

        if !still_connected {
            self.presence.user_offline(info.workspace_id, info.user_id);
            let msg = WsMessage::PresenceUpdate {
                user: info.user_id,
                status: "offline".into(),
            };
            self.send_to_workspace(info.workspace_id, &msg, None);
        }
    }

    async fn handle_join_doc(&mut self, conn_id: Uuid, doc_id: Uuid) {
        let Some(info) = self.connections.get(&conn_id) else {
            return;
        };
        let workspace_id = info.workspace_id;

        self.doc_rooms.entry(doc_id).or_default().insert(conn_id);
        self.conn_docs.entry(conn_id).or_default().insert(doc_id);

        match self.crdt.get_or_load(doc_id, workspace_id).await {
            Ok(state) => {
                let msg = WsMessage::CrdtState {
                    doc_id,
                    payload: BASE64.encode(&state),
                };
                self.send_to_conn(conn_id, &msg);
            }
            Err(e) => {
                tracing::error!(doc_id = %doc_id, "failed to load crdt doc: {e:?}");
            }
        }
    }

    fn handle_leave_doc(&mut self, conn_id: Uuid, doc_id: Uuid) {
        if let Some(conns) = self.doc_rooms.get_mut(&doc_id) {
            conns.remove(&conn_id);
            if conns.is_empty() {
                self.doc_rooms.remove(&doc_id);
            }
        }
        if let Some(docs) = self.conn_docs.get_mut(&conn_id) {
            docs.remove(&doc_id);
        }
    }

    async fn handle_incoming(&mut self, conn_id: Uuid, message: WsMessage) {
        let Some(info) = self.connections.get(&conn_id) else {
            return;
        };
        let user_id = info.user_id;
        let workspace_id = info.workspace_id;

        match message {
            WsMessage::CrdtJoin { doc_id } => {
                self.handle_join_doc(conn_id, doc_id).await;
            }
            WsMessage::CrdtLeave { doc_id } => {
                self.handle_leave_doc(conn_id, doc_id);
            }
            WsMessage::CrdtSync { doc_id, ref payload } => {
                if let Err(e) = self.crdt.apply_update(doc_id, workspace_id, payload).await {
                    tracing::error!(doc_id = %doc_id, "crdt apply failed: {e:?}");
                    return;
                }
                self.send_to_doc(doc_id, &message, Some(conn_id));
            }
            WsMessage::CrdtAwareness { doc_id, .. } => {
                self.send_to_doc(doc_id, &message, Some(conn_id));
            }
            WsMessage::ChatMessage { channel_id, ref text, .. } => {
                let broadcast = WsMessage::ChatMessage {
                    channel_id,
                    text: text.clone(),
                    user: user_id,
                };
                self.send_to_workspace(workspace_id, &broadcast, None);
            }
            _ => {}
        }
    }

    fn send_to_workspace(&self, workspace_id: Uuid, message: &WsMessage, exclude: Option<Uuid>) {
        let Some(conns) = self.workspace_rooms.get(&workspace_id) else {
            return;
        };
        let json = match serde_json::to_string(message) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("failed to serialize ws message: {e}");
                return;
            }
        };
        for &conn_id in conns {
            if exclude == Some(conn_id) {
                continue;
            }
            if let Some(info) = self.connections.get(&conn_id) {
                let _ = info.tx.send(json.clone());
            }
        }
    }

    fn send_to_doc(&self, doc_id: Uuid, message: &WsMessage, exclude: Option<Uuid>) {
        let Some(conns) = self.doc_rooms.get(&doc_id) else {
            return;
        };
        let json = match serde_json::to_string(message) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("failed to serialize ws message: {e}");
                return;
            }
        };
        for &conn_id in conns {
            if exclude == Some(conn_id) {
                continue;
            }
            if let Some(info) = self.connections.get(&conn_id) {
                let _ = info.tx.send(json.clone());
            }
        }
    }

    fn send_to_conn(&self, conn_id: Uuid, message: &WsMessage) {
        if let Some(info) = self.connections.get(&conn_id) {
            if let Ok(json) = serde_json::to_string(message) {
                let _ = info.tx.send(json);
            }
        }
    }
}
