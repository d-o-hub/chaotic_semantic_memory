use std::env;
use std::time::Instant;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};

const NS: &str = "_default";

const DEFAULT_TURSO_POOL_SIZE: usize = 4;
const DEFAULT_TURSO_ROUNDTRIP_SAMPLES: usize = 25;
const DEFAULT_TURSO_ROUNDTRIP_MAX_P50_MS: f64 = 20.0;

fn p50_ms(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

#[tokio::test]
async fn turso_roundtrip_p50_under_20ms_when_configured() {
    let Some(url) = non_empty_env("TURSO_DATABASE_URL") else {
        println!("Skipping Turso latency test: TURSO_DATABASE_URL is not set");
        return;
    };
    let Some(token) = non_empty_env("TURSO_AUTH_TOKEN") else {
        println!("Skipping Turso latency test: TURSO_AUTH_TOKEN is not set");
        return;
    };

    let pool_size = env_usize("CSM_TURSO_POOL_SIZE", DEFAULT_TURSO_POOL_SIZE);
    let sample_count = env_usize(
        "CSM_TURSO_ROUNDTRIP_SAMPLES",
        DEFAULT_TURSO_ROUNDTRIP_SAMPLES,
    );
    let threshold_ms = env_f64(
        "CSM_TURSO_ROUNDTRIP_MAX_P50_MS",
        DEFAULT_TURSO_ROUNDTRIP_MAX_P50_MS,
    );

    let persistence = Persistence::new_turso_with_pool(&url, &token, pool_size)
        .await
        .expect("new_turso_with_pool");

    let id = format!("turso-rt-{}", chrono_like_now_nanos());
    let concept = ConceptBuilder::new(id.clone())
        .with_vector(HVec10240::random())
        .build()
        .expect("concept");
    persistence
        .save_concept(NS, &concept)
        .await
        .expect("save_concept");

    let mut durations_ms = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let start = Instant::now();
        let loaded = persistence
            .load_concept(NS, &id)
            .await
            .expect("load_concept");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(loaded.is_some(), "concept should exist");
        durations_ms.push(elapsed);
    }

    let p50 = p50_ms(&mut durations_ms);
    println!("TURSO_ROUNDTRIP_P50_MS={p50:.3}");
    assert!(
        p50 < threshold_ms,
        "turso p50={p50:.3}ms is above {threshold_ms}ms"
    );
}

fn chrono_like_now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}
