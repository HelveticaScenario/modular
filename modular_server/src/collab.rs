use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use ts_rs::TS;
use yrs::sync::awareness::AwarenessUpdateEntry;
use yrs::sync::{Awareness, AwarenessUpdate};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct CollabInitPayload {
    pub doc_id: String,
    /// Serialized yrs document update (v1) that brings a client to current state.
    pub update: Vec<u8>,
    /// Serialized awareness update (v1) containing all known client states.
    pub awareness: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CollabBroadcast {
    pub doc_id: String,
    pub origin_client_id: Option<String>,
    pub message: crate::protocol::OutputMessage,
}

#[derive(Debug)]
struct DocState {
    awareness: Awareness,
}

impl DocState {
    fn new() -> Self {
        Self {
            awareness: Awareness::new(Doc::new()),
        }
    }
}

#[derive(Default)]
pub struct CollabEngine {
    docs: Mutex<HashMap<String, Arc<Mutex<DocState>>>>,
    next_awareness_id: AtomicU64,
}

impl CollabEngine {
    pub fn new() -> Self {
        Self {
            docs: Mutex::new(HashMap::new()),
            next_awareness_id: AtomicU64::new(rand::random::<u64>().max(1)),
        }
    }

    pub fn next_client_awareness_id(&self) -> u64 {
        self.next_awareness_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn doc(&self, doc_id: &str) -> Arc<Mutex<DocState>> {
        let mut docs = self.docs.lock().await;
        docs.entry(doc_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(DocState::new())))
            .clone()
    }

    pub async fn join(
        &self,
        doc_id: &str,
        initial_awareness: Option<Vec<u8>>,
    ) -> Result<CollabInitPayload> {
        let doc = self.doc(doc_id).await;
        let guard = doc.lock().await;

        if let Some(bytes) = initial_awareness {
            let update =
                AwarenessUpdate::decode_v1(&bytes).context("decode initial awareness update")?;
            guard
                .awareness
                .apply_update(update)
                .context("apply initial awareness update")?;
        }

        let txn = guard.awareness.doc().transact();
        let update = txn.encode_diff_v1(&StateVector::default());
        let awareness = guard
            .awareness
            .update()
            .context("encode awareness snapshot")?
            .encode_v1();

        Ok(CollabInitPayload {
            doc_id: doc_id.to_string(),
            update,
            awareness,
        })
    }

    pub async fn apply_update(&self, doc_id: &str, update: &[u8]) -> Result<()> {
        let doc = self.doc(doc_id).await;
        let guard = doc.lock().await;
        let parsed = Update::decode_v1(update).context("decode incoming yrs update")?;
        let mut txn = guard.awareness.doc().transact_mut();
        txn.apply_update(parsed)
            .context("apply incoming yrs update")?;
        Ok(())
    }

    pub async fn apply_awareness(&self, doc_id: &str, update: &[u8]) -> Result<()> {
        let doc = self.doc(doc_id).await;
        let guard = doc.lock().await;
        let parsed =
            AwarenessUpdate::decode_v1(update).context("decode incoming awareness update")?;
        guard
            .awareness
            .apply_update(parsed)
            .context("apply awareness update")?;
        Ok(())
    }

    pub async fn remove_client_awareness(
        &self,
        doc_id: &str,
        client_id: u64,
    ) -> Result<Option<Vec<u8>>> {
        let doc = self.doc(doc_id).await;
        let guard = doc.lock().await;
        let (clock, _) = match guard.awareness.meta(client_id) {
            Some(meta) => meta,
            None => return Ok(None),
        };
        let mut clients = HashMap::new();
        clients.insert(
            client_id,
            AwarenessUpdateEntry {
                clock: clock + 1,
                json: Arc::from("null"),
            },
        );
        let update = AwarenessUpdate { clients };
        guard
            .awareness
            .apply_update(update.clone())
            .context("apply removal awareness update")?;
        Ok(Some(update.encode_v1()))
    }
}
