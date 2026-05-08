#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Exact search via linear scan.

use std::collections::HashMap;
use crate::error::Result;
use crate::hyperdim::Hypervector;
use crate::index::{AnnIndex, IndexStats};
use crate::singularity::Concept;

#[derive(Debug)]
pub struct BruteForce<H: Hypervector> {
    indices: Vec<String>,
    vectors: Vec<H>,
    id_to_index: HashMap<String, usize>,
}

impl<H: Hypervector> BruteForce<H> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<H: Hypervector> AnnIndex<H> for BruteForce<H> {
    fn insert(&mut self, id: String, vec: &H) -> Result<()> {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.vectors[idx] = *vec;
        } else {
            let idx = self.indices.len();
            self.id_to_index.insert(id.clone(), idx);
            self.indices.push(id);
            self.vectors.push(*vec);
        }
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if let Some(idx) = self.id_to_index.remove(id) {
            self.indices.swap_remove(idx);
            let _ = self.vectors.swap_remove(idx);
            if idx < self.indices.len() {
                let swapped_id = &self.indices[idx];
                self.id_to_index.insert(swapped_id.clone(), idx);
            }
        }
        Ok(())
    }

    fn search(&self, query: &H, top_k: usize) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.indices.is_empty() { return Ok(Vec::new()); }
        let mut scores: Vec<(usize, f32)> = self.vectors.iter().enumerate()
            .map(|(idx, v)| (idx, query.cosine_similarity(v))).collect();
        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        Ok(scores.into_iter().map(|(idx, sim)| (self.indices[idx].clone(), sim)).collect())
    }

    fn search_filtered(&self, query: &H, top_k: usize, filter: &crate::metadata_filter::MetadataFilter, concepts: &HashMap<String, Concept<H>>) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.indices.is_empty() { return Ok(Vec::new()); }
        let mut scores: Vec<(usize, f32)> = self.indices.iter().enumerate()
            .filter(|(_, id)| concepts.get(*id).is_some_and(|c| filter.matches(&c.metadata)))
            .map(|(idx, _)| (idx, query.cosine_similarity(&self.vectors[idx]))).collect();
        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        Ok(scores.into_iter().map(|(idx, sim)| (self.indices[idx].clone(), sim)).collect())
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept<H>>) -> Result<()> {
        self.indices.clear(); self.vectors.clear(); self.id_to_index.clear();
        for (id, concept) in concepts { self.insert(id.clone(), &concept.vector)?; }
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        IndexStats { backend: "BruteForce".to_string(), count: self.indices.len(), memory_usage_bytes: self.indices.len() * (std::mem::size_of::<String>() + std::mem::size_of::<H>() + 16) }
    }

    fn serialize(&self) -> Result<Vec<u8>> { Ok(Vec::new()) }
    fn deserialize(&mut self, _data: &[u8]) -> Result<()> { Ok(()) }
}

impl<H: Hypervector> Default for BruteForce<H> {
    fn default() -> Self { Self { indices: Vec::new(), vectors: Vec::new(), id_to_index: HashMap::new() } }
}
