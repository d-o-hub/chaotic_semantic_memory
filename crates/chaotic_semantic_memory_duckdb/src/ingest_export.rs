use crate::Analytics;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IngestReport {
    pub concepts_loaded: usize,
    pub associations_loaded: usize,
    pub benchmarks_loaded: usize,
}

#[derive(Debug, Deserialize)]
struct ExportPayloadStub {
    pub concepts: Vec<serde_json::Value>,
    pub associations: Vec<(String, String, f64)>,
}

impl Analytics {
    pub fn load_export_json<P: AsRef<Path>>(&mut self, path: P) -> Result<IngestReport> {
        let file = std::fs::File::open(path)?;
        let payload: ExportPayloadStub = serde_json::from_reader(file)?;

        let tx = self.conn.transaction()?;

        let mut concepts_loaded = 0;
        for concept in &payload.concepts {
            let id = concept["id"].as_str().ok_or_else(|| {
                crate::error::AnalyticsError::InvalidInput("Missing concept id".to_string())
            })?;
            // text is optional or might be in metadata depending on version
            let text = concept["text"]
                .as_str()
                .or(concept["metadata"]["text"].as_str());
            let namespace = concept["namespace"].as_str().unwrap_or("default");
            let created_at_us = concept["created_at"].as_i64().unwrap_or(0);
            let updated_at_us = concept["modified_at"].as_i64().unwrap_or(created_at_us);
            let expires_at_us = concept["expires_at"].as_i64();
            let metadata_json = serde_json::to_string(&concept["metadata"])?;

            tx.execute(
                "INSERT OR REPLACE INTO concepts (id, text, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![id, text, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json],
            )?;
            concepts_loaded += 1;
        }

        let mut associations_loaded = 0;
        for (src_id, dst_id, strength) in &payload.associations {
            tx.execute(
                "INSERT OR REPLACE INTO associations (src_id, dst_id, strength) VALUES (?, ?, ?)",
                duckdb::params![src_id, dst_id, strength],
            )?;
            associations_loaded += 1;
        }

        tx.commit()?;

        Ok(IngestReport {
            concepts_loaded,
            associations_loaded,
            ..Default::default()
        })
    }
}
