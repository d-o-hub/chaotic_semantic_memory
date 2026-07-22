use crate::index::AnnIndex;
use crate::singularity::{Concept, SingularityConfig};
use crate::singularity_cache::{CacheMetrics, QueryCache};
use crate::singularity_retrieval::RetrievalStats;
use csm_core_lib::hyperdim::{HVec10240, Hypervector};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// State for a single namespace in the singularity engine.
#[derive(Debug)]
pub struct NamespaceState<H: Hypervector = HVec10240> {
    pub concepts: HashMap<String, Concept<H>>,
    pub associations: HashMap<String, HashMap<String, (f32, u64)>>,
    pub(crate) concept_indices: Vec<String>,
    pub(crate) concept_vectors: Vec<H>,
    pub(crate) id_to_index: HashMap<String, usize>,
    pub(crate) query_cache: RwLock<QueryCache>,
    pub(crate) cache_metrics: Arc<CacheMetrics>,
    pub(crate) last_retrieval_stats: RwLock<RetrievalStats>,
    pub index: Box<dyn AnnIndex<H>>,
}

impl<H: Hypervector> NamespaceState<H> {
    pub fn new(
        config: &SingularityConfig,
        index: Box<dyn AnnIndex<H>>,
        cache_metrics: Arc<CacheMetrics>,
    ) -> Self {
        Self {
            concepts: HashMap::new(),
            associations: HashMap::new(),
            concept_indices: Vec::new(),
            concept_vectors: Vec::new(),
            id_to_index: HashMap::new(),
            query_cache: RwLock::new(QueryCache::with_capacity(config.concept_cache_size)),
            cache_metrics,
            last_retrieval_stats: RwLock::new(RetrievalStats::default()),
            index,
        }
    }
}
