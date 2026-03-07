//! CRDT document manager backed by yrs.
//!
//! ## CrdtManager
//!
//! Manages yrs `Doc` instances in memory with lazy loading from PostgreSQL.
//! Each document is identified by a UUID and scoped to a workspace.
//!
//! ## get_or_load
//!
//! Returns the full encoded state of a document. Loads from the database
//! on first access, or creates a new empty document if none exists.
//!
//! ## apply_update
//!
//! Decodes a base64 yrs update, applies it to the in-memory document,
//! and persists the updated state to the database.
//!
//! ## encode_state
//!
//! Returns the full state of an in-memory document as bytes, or `None`
//! if the document is not loaded.

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sqlx::PgPool;
use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::{Doc, ReadTxn, Transact, Update};

pub struct CrdtManager {
    docs: HashMap<Uuid, Doc>,
    pool: PgPool,
}

impl CrdtManager {
    pub fn new(pool: PgPool) -> Self {
        Self {
            docs: HashMap::new(),
            pool,
        }
    }

    async fn ensure_loaded(&mut self, doc_id: Uuid) -> eyre::Result<()> {
        if self.docs.contains_key(&doc_id) {
            return Ok(());
        }

        let doc = Doc::new();

        if let Some(row) = kahf_db::crdt_repo::get_crdt_doc(&self.pool, doc_id).await? {
            let update = Update::decode_v1(&row.state)
                .map_err(|e| eyre::eyre!("failed to decode crdt state: {e}"))?;
            let mut txn = doc.transact_mut();
            txn.apply_update(update)
                .map_err(|e| eyre::eyre!("failed to apply crdt state: {e}"))?;
        }

        self.docs.insert(doc_id, doc);
        Ok(())
    }

    fn encode_doc_state(&self, doc_id: Uuid) -> Vec<u8> {
        let doc = self.docs.get(&doc_id).unwrap();
        let txn = doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    }

    pub async fn get_or_load(&mut self, doc_id: Uuid, workspace_id: Uuid) -> eyre::Result<Vec<u8>> {
        let _ = workspace_id;
        self.ensure_loaded(doc_id).await?;
        Ok(self.encode_doc_state(doc_id))
    }

    pub async fn apply_update(
        &mut self,
        doc_id: Uuid,
        workspace_id: Uuid,
        payload_b64: &str,
    ) -> eyre::Result<()> {
        let bytes = BASE64.decode(payload_b64)
            .map_err(|e| eyre::eyre!("invalid base64 payload: {e}"))?;

        self.ensure_loaded(doc_id).await?;

        let doc = self.docs.get(&doc_id).unwrap();
        {
            let update = Update::decode_v1(&bytes)
                .map_err(|e| eyre::eyre!("invalid yrs update: {e}"))?;
            let mut txn = doc.transact_mut();
            txn.apply_update(update)
                .map_err(|e| eyre::eyre!("failed to apply update: {e}"))?;
        }

        let state = self.encode_doc_state(doc_id);
        kahf_db::crdt_repo::save_crdt_doc(&self.pool, doc_id, workspace_id, &state).await?;

        Ok(())
    }

    pub fn encode_state(&self, doc_id: Uuid) -> Option<Vec<u8>> {
        if !self.docs.contains_key(&doc_id) {
            return None;
        }
        Some(self.encode_doc_state(doc_id))
    }
}
