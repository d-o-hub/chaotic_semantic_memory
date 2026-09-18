//! Measured memory/storage model.
//!
//! Each scale point is measured in a fresh child process (`--memory-point`)
//! so RSS deltas are not polluted by the previous point's allocator state.
//! The parent fits `bytes = intercept + slope * concepts` on the fit points,
//! reports the held-out error of the remaining point, and projects 10M.

use std::time::Instant;

use serde_json::{Value, json};

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};

use super::util::{Corpus, linear_fit, rss_bytes, sqlite_bytes};

const NS: &str = "_default";

/// Parameters for the memory model run.
pub struct MemoryParams {
    /// Concept counts used to fit the model.
    pub fit: Vec<usize>,
    /// Concept count held out to score the model.
    pub holdout: usize,
    /// Concept count the projection targets.
    pub projection: u64,
    /// Claim under evaluation, in bytes (12 MiB by default).
    pub claim_bytes: u64,
}

/// Measure one scale point in this process (called for each child run).
pub async fn measure_point(concepts: usize) -> Value {
    let baseline_rss = rss_bytes();

    let mut engine = Singularity::with_config(SingularityConfig::default());
    let inject_start = Instant::now();
    for concept in Corpus::stream_concepts(42, concepts, 64, 512) {
        engine.inject(NS, concept).expect("inject");
    }
    let inject_ms = inject_start.elapsed().as_secs_f64() * 1000.0;

    let peak_rss = super::util::peak_rss_bytes();
    let after_rss = rss_bytes();

    // Persisted footprint of the same corpus, measured in a fresh database.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let db = dir.path().join("memory.db");
    let path = db.to_str().expect("utf8 path").to_string();
    let persistence = Persistence::new_local(&path).await.expect("open db");
    let mut batch: Vec<Concept> = Vec::with_capacity(500);
    for concept in Corpus::stream_concepts(42, concepts, 64, 512) {
        batch.push(concept);
        if batch.len() == 500 {
            persistence
                .save_concepts(NS, &batch)
                .await
                .expect("save batch");
            batch.clear();
        }
    }
    if !batch.is_empty() {
        persistence
            .save_concepts(NS, &batch)
            .await
            .expect("save batch");
    }
    persistence.checkpoint().await.expect("checkpoint");
    let storage = sqlite_bytes(&db);

    json!({
        "concepts": concepts,
        "baseline_rss_bytes": baseline_rss,
        "after_rss_bytes": after_rss,
        "rss_delta_bytes": after_rss.saturating_sub(baseline_rss),
        "peak_rss_bytes": peak_rss,
        "rss_bytes_per_concept": after_rss.saturating_sub(baseline_rss) as f64 / concepts as f64,
        "peak_bytes_per_concept": peak_rss as f64 / concepts as f64,
        "inject_ms": inject_ms,
        "storage": storage,
        "storage_bytes_per_concept": storage["total_bytes"].as_u64().unwrap_or(0) as f64 / concepts as f64,
    })
}

/// Fit and project from previously measured points.
pub fn model(params: &MemoryParams, points: &[Value]) -> Value {
    let fit_points: Vec<(f64, f64)> = points
        .iter()
        .filter(|p| {
            let n = p["concepts"].as_u64().unwrap_or(0) as usize;
            n != params.holdout
        })
        .map(|p| {
            (
                p["concepts"].as_u64().unwrap_or(0) as f64,
                p["rss_delta_bytes"].as_u64().unwrap_or(0) as f64,
            )
        })
        .collect();
    let storage_points: Vec<(f64, f64)> = points
        .iter()
        .filter(|p| {
            let n = p["concepts"].as_u64().unwrap_or(0) as usize;
            n != params.holdout
        })
        .map(|p| {
            (
                p["concepts"].as_u64().unwrap_or(0) as f64,
                p["storage"]["total_bytes"].as_u64().unwrap_or(0) as f64,
            )
        })
        .collect();

    let (rss_intercept, rss_slope) = linear_fit(&fit_points);
    let (storage_intercept, storage_slope) = linear_fit(&storage_points);

    let holdout = points
        .iter()
        .find(|p| p["concepts"].as_u64().unwrap_or(0) as usize == params.holdout);
    let (holdout_rss_error, holdout_storage_error) = match holdout {
        Some(p) => {
            let n = p["concepts"].as_u64().unwrap_or(0) as f64;
            let actual_rss = p["rss_delta_bytes"].as_u64().unwrap_or(0) as f64;
            let actual_storage = p["storage"]["total_bytes"].as_u64().unwrap_or(0) as f64;
            let predicted_rss = rss_intercept + rss_slope * n;
            let predicted_storage = storage_intercept + storage_slope * n;
            (
                relative_error(predicted_rss, actual_rss),
                relative_error(predicted_storage, actual_storage),
            )
        }
        None => (f64::NAN, f64::NAN),
    };

    let projected = params.projection as f64;
    let projected_rss = rss_intercept + rss_slope * projected;
    let projected_storage = storage_intercept + storage_slope * projected;
    let claim = params.claim_bytes as f64;

    json!({
        "kind": "memory_model",
        "corpus_version": super::util::CORPUS_VERSION,
        "projection_concepts": params.projection,
        "claim_bytes": params.claim_bytes,
        "fit": {
            "points": params.fit.len(),
            "fit_concepts": params.fit,
            "rss_intercept_bytes": rss_intercept,
            "rss_bytes_per_concept": rss_slope,
            "storage_intercept_bytes": storage_intercept,
            "storage_bytes_per_concept": storage_slope,
        },
        "holdout": {
            "concepts": params.holdout,
            "rss_relative_error": holdout_rss_error,
            "storage_relative_error": holdout_storage_error,
            "within_5_percent": holdout_rss_error <= 0.05 && holdout_storage_error <= 0.05,
        },
        "projection": {
            "rss_bytes": projected_rss,
            "storage_bytes": projected_storage,
            "claim_supported": projected_rss <= claim,
            "claim_ratio_rss": projected_rss / claim,
            "claim_ratio_storage": projected_storage / claim,
        },
        "points": points,
    })
}

fn relative_error(predicted: f64, actual: f64) -> f64 {
    if actual.abs() < f64::EPSILON {
        return f64::NAN;
    }
    (predicted - actual).abs() / actual
}
