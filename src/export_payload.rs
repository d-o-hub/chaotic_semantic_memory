use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExportPayload {
    pub(crate) version: String,
    pub(crate) exported_at: u64,
    pub(crate) concepts: Vec<crate::singularity::Concept>,
    pub(crate) associations: Vec<(String, String, f32)>,
}

pub(crate) fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
