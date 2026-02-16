use serde::{Deserialize, Serialize};

use crate::framework::MemoryError;
use crate::hyperdim::HVec10240;

#[cfg(not(target_arch = "wasm32"))]
pub type TursoClient = turso_client::Client;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
pub struct TursoClient;

#[derive(Clone, Serialize, Deserialize)]
pub struct ConceptRow {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl ConceptRow {
    pub fn from_pair(name: String, h: HVec10240) -> Self {
        Self {
            name,
            bytes: h.as_bytes().to_vec(),
        }
    }

    pub fn into_pair(self) -> Result<(String, HVec10240), MemoryError> {
        let hv = HVec10240::from_bytes(&self.bytes)
            .ok_or_else(|| MemoryError::Serialization("invalid hypervector bytes".to_string()))?;
        Ok((self.name, hv))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn ensure_schema(client: &TursoClient) -> Result<(), MemoryError> {
    let _ = client
        .execute(
            "CREATE TABLE IF NOT EXISTS concepts(name TEXT PRIMARY KEY, payload BLOB NOT NULL)",
            (),
        )
        .await
        .map_err(|e| MemoryError::Db(e.to_string()))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn ensure_schema(_client: &TursoClient) -> Result<(), MemoryError> {
    Ok(())
}
