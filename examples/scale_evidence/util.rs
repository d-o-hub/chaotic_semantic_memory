//! Shared helpers for the ADR-0095 scale-evidence runner: deterministic
//! corpora, percentile aggregation, and process/storage sizing.

use rand::RngExt;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::{Value, json};

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::singularity::Concept;

/// Corpus generator identity, recorded in every artifact (ADR-0095 requires a
/// dataset/corpus version).
pub const CORPUS_VERSION: &str = "synthetic-clustered-v1";

/// Deterministic clustered corpus.
///
/// Vectors are `clusters` random cluster centres; concept `i` belongs to
/// cluster `i % clusters` and is the centre with `noise_bits` random bits
/// flipped. Queries use the same centres with `noise_bits / 2` flips, so a
/// nearest-neighbour ground truth exists and approximate indexes can be
/// scored against it. Generation is prefix-stable: the first `k` concepts of
/// an `N`-concept corpus are identical for every `N`, because the RNG stream
/// advances by a fixed number of draws per concept.
pub struct Corpus {
    /// Concept vectors, index-aligned with [`Corpus::ids`].
    pub vectors: Vec<HVec10240>,
    /// Concept ids (`c-<index>`).
    pub ids: Vec<String>,
}

impl Corpus {
    /// Generate `count` concepts and `queries` query vectors.
    pub fn generate(
        seed: u64,
        count: usize,
        queries: usize,
        clusters: usize,
        noise_bits: usize,
    ) -> (Self, Vec<HVec10240>) {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let clusters = clusters.max(1);
        let centres: Vec<HVec10240> = (0..clusters).map(|_| random_vector(&mut rng)).collect();

        let mut vectors = Vec::with_capacity(count);
        let mut ids = Vec::with_capacity(count);
        for i in 0..count {
            let centre = &centres[i % clusters];
            vectors.push(flip_bits(&mut rng, centre, noise_bits));
            ids.push(format!("c-{i}"));
        }

        let query_vecs = (0..queries)
            .map(|j| {
                let centre = &centres[j % clusters];
                flip_bits(&mut rng, centre, noise_bits / 2)
            })
            .collect();

        (Self { vectors, ids }, query_vecs)
    }

    /// Stored concepts, for indexes that rebuild from concept maps.
    pub fn concepts(&self) -> Vec<Concept> {
        (0..self.vectors.len())
            .map(|i| Concept {
                id: self.ids[i].clone(),
                vector: self.vectors[i],
                metadata: Default::default(),
                created_at: 1,
                modified_at: 1,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            })
            .collect()
    }

    /// Stream concepts one at a time, holding only the cluster centres.
    ///
    /// Memory-model measurements must not retain a second copy of the corpus
    /// in the measuring process, so this generator produces each concept on
    /// demand and the caller drops it after injecting or saving it.
    pub fn stream_concepts(
        seed: u64,
        count: usize,
        clusters: usize,
        noise_bits: usize,
    ) -> ConceptStream {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let clusters = clusters.max(1);
        let centres: Vec<HVec10240> = (0..clusters).map(|_| random_vector(&mut rng)).collect();
        ConceptStream {
            rng,
            centres,
            clusters,
            noise_bits,
            next: 0,
            count,
        }
    }
}

/// Iterator produced by [`Corpus::stream_concepts`].
pub struct ConceptStream {
    rng: ChaCha8Rng,
    centres: Vec<HVec10240>,
    clusters: usize,
    noise_bits: usize,
    next: usize,
    count: usize,
}

impl Iterator for ConceptStream {
    type Item = Concept;

    fn next(&mut self) -> Option<Concept> {
        if self.next >= self.count {
            return None;
        }
        let i = self.next;
        self.next += 1;
        let centre = &self.centres[i % self.clusters];
        let vector = flip_bits(&mut self.rng, centre, self.noise_bits);
        Some(Concept {
            id: format!("c-{i}"),
            vector,
            metadata: Default::default(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        })
    }
}

fn random_vector(rng: &mut ChaCha8Rng) -> HVec10240 {
    let mut v = HVec10240::zero();
    for word in &mut v.data {
        *word = ((rng.random::<u64>() as u128) << 64) | rng.random::<u64>() as u128;
    }
    v
}

fn flip_bits(rng: &mut ChaCha8Rng, base: &HVec10240, bits: usize) -> HVec10240 {
    let mut v = *base;
    for _ in 0..bits {
        let bit = rng.random_range(0..HVec10240::DIMENSION);
        v.data[bit / 128] ^= 1u128 << (bit % 128);
    }
    v
}

/// Latency distribution in microseconds.
pub fn summarize_us(mut samples: Vec<u64>) -> Value {
    if samples.is_empty() {
        return json!({ "n": 0 });
    }
    samples.sort_unstable();
    let pct = |p: usize| -> u64 {
        let idx = (samples.len() * p / 100).min(samples.len() - 1);
        samples[idx]
    };
    let mean = samples.iter().sum::<u64>() as f64 / samples.len() as f64;
    json!({
        "n": samples.len(),
        "min_us": samples[0],
        "p50_us": pct(50),
        "p95_us": pct(95),
        "p99_us": pct(99),
        "max_us": samples[samples.len() - 1],
        "mean_us": mean,
    })
}

/// Resident set size in bytes, from `/proc/self/statm` (pages * page size).
pub fn rss_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Peak resident set size in bytes (`VmHWM`), when the kernel reports it.
pub fn peak_rss_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            if let Some(kb) = rest.split_whitespace().next() {
                return kb.parse::<u64>().unwrap_or(0) * 1024;
            }
        }
    }
    0
}

/// Size in bytes of a file, or 0 when absent.
pub fn file_bytes(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// `db`, `db-wal` and `db-shm` sizes: ADR-0095 requires persisted DB, WAL and
/// index bytes reported separately.
pub fn sqlite_bytes(db: &std::path::Path) -> Value {
    let wal = std::path::PathBuf::from(format!("{}-wal", db.display()));
    let shm = std::path::PathBuf::from(format!("{}-shm", db.display()));
    let db_bytes = file_bytes(db);
    let wal_bytes = file_bytes(&wal);
    let shm_bytes = file_bytes(&shm);
    json!({
        "db_bytes": db_bytes,
        "wal_bytes": wal_bytes,
        "shm_bytes": shm_bytes,
        "total_bytes": db_bytes + wal_bytes + shm_bytes,
    })
}

/// Least-squares fit of `y = intercept + slope * x`.
pub fn linear_fit(points: &[(f64, f64)]) -> (f64, f64) {
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();
    let denom = n * sum_xx - sum_x * sum_x;
    let slope = if denom.abs() < f64::EPSILON {
        0.0
    } else {
        (n * sum_xy - sum_x * sum_y) / denom
    };
    let intercept = (sum_y - slope * sum_x) / n;
    (intercept, slope)
}

/// FNV-1a over the corpus bytes: a cheap, dependency-free corpus checksum.
pub fn corpus_checksum(vectors: &[HVec10240]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for v in vectors {
        for word in &v.data {
            for byte in word.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x1000_0000_01b3);
            }
        }
    }
    format!("fnv1a64:{hash:016x}")
}
