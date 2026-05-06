use crate::hyperdim::{HVec10240, Hypervector};
use crate::index::AnnIndex;
use crate::singularity::{Concept, SingularityConfig};
use crate::singularity_cache::{CacheMetrics, QueryCache};
use crate::singularity_retrieval::RetrievalStats;
use std::collections::HashMap;
use std::sync::RwLock;

/// State for a single namespace in the singularity engine.
#[derive(Debug)]
pub struct NamespaceState<H: Hypervector = HVec10240> {
    pub(crate) concepts: HashMap<String, Concept<H>>,
    pub(crate) associations: HashMap<String, HashMap<String, f32>>,
    pub(crate) concept_indices: Vec<String>,
    pub(crate) concept_vectors: Vec<H>,
    pub(crate) id_to_index: HashMap<String, usize>,
    pub(crate) query_cache: RwLock<QueryCache>,
    pub(crate) cache_metrics: CacheMetrics,
    pub(crate) last_retrieval_stats: RwLock<RetrievalStats>,
    pub(crate) index: Box<dyn AnnIndex<H>>,
}

impl<H: Hypervector> NamespaceState<H> {
    pub fn new(config: &SingularityConfig, index: Box<dyn AnnIndex<H>>) -> Self {
        Self {
            concepts: HashMap::new(),
            associations: HashMap::new(),
            concept_indices: Vec::new(),
            concept_vectors: Vec::new(),
            id_to_index: HashMap::new(),
            query_cache: RwLock::new(QueryCache::with_capacity(config.concept_cache_size)),
            cache_metrics: CacheMetrics::default(),
            last_retrieval_stats: RwLock::new(RetrievalStats::default()),
            index,
        }
    }
}
