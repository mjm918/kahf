//! In-process event bus using `tokio::sync::broadcast`.
//!
//! ## BroadcastEventBus
//!
//! Implements the `EventBus` trait from `kahf-core`. Replaces Kafka/Redpanda
//! with a zero-overhead in-process broadcast channel. All subscribers
//! receive every event. Slow consumers that fall behind lose events
//! (broadcast channel semantics). Publish succeeds even with no subscribers.
//!
//! ## new
//!
//! Creates a new event bus with the given channel capacity. A capacity of
//! 1024 is recommended for most workloads.
//!
//! ## subscribe
//!
//! Returns a new `broadcast::Receiver<Event>` for consuming published events.
//! Each subscriber gets an independent cursor into the broadcast stream.

use kahf_core::event::Event;
use kahf_core::traits::EventBus;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct BroadcastEventBus {
    tx: broadcast::Sender<Event>,
}

impl BroadcastEventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl EventBus for BroadcastEventBus {
    async fn publish(&self, event: Event) -> kahf_core::Result<()> {
        let _ = self.tx.send(event);
        Ok(())
    }
}
