//! WebSocket connection handler.
//!
//! ## handle_connection
//!
//! Manages a single WebSocket connection lifecycle. Registers the
//! connection with the hub, then spawns two concurrent tasks:
//!
//! - **Read task:** reads JSON messages from the WebSocket, deserializes
//!   them as `WsMessage`, and forwards to the hub via `Hub::incoming`.
//! - **Write task:** reads JSON strings from the hub's outbound channel
//!   and sends them as WebSocket text frames.
//!
//! When either task completes (client disconnect or send failure),
//! the connection is cleaned up and the hub is notified via
//! `Hub::disconnect`.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::hub::Hub;
use crate::protocol::WsMessage;

pub async fn handle_connection(
    socket: WebSocket,
    hub: Hub,
    user_id: Uuid,
    workspace_id: Uuid,
) {
    let conn_id = Uuid::new_v4();
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();

    hub.connect(conn_id, user_id, workspace_id, outbound_tx);

    let (mut ws_sink, mut ws_stream) = socket.split();

    let hub_read = hub.clone();
    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => hub_read.incoming(conn_id, ws_msg),
                        Err(e) => tracing::warn!(conn_id = %conn_id, "invalid ws message: {e}"),
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let write_task = tokio::spawn(async move {
        while let Some(json) = outbound_rx.recv().await {
            if ws_sink.send(Message::text(json)).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = read_task => {}
        _ = write_task => {}
    }

    hub.disconnect(conn_id);
}
