use std::sync::Arc;
use tokio::fs;
use tracing::warn;

use crate::error::Result;
use crate::export_payload::{unix_now_secs, ExportPayload};
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::singularity::ConceptBuilder;

impl ChaoticSemanticFramework {
    pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let mut to_save = Vec::with_capacity(concepts.len());
        {
            let mut sing = self.singularity.write().await;
            for (id, vector) in concepts {
                Self::validate_concept_id(id)?;
                let concept = ConceptBuilder::new(id.clone())
                    .with_vector(*vector)
                    .build()?;
                sing.inject(concept.clone())?;
                to_save.push(concept);
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concepts(&to_save).await?;
        }

        self.metrics.inc_concepts_injected(to_save.len() as u64);
        Ok(())
    }

    pub async fn associate_many(&self, associations: &[(String, String, f32)]) -> Result<()> {
        if associations.is_empty() {
            return Ok(());
        }

        {
            let mut sing = self.singularity.write().await;
            for (from, to, strength) in associations {
                Self::validate_concept_id(from)?;
                Self::validate_concept_id(to)?;
                Self::validate_association_strength(*strength)?;
                sing.associate(from, to, *strength)?;
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_associations(associations).await?;
        }

        self.metrics
            .inc_associations_created(associations.len() as u64);
        Ok(())
    }

    pub async fn probe_batch(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Vec<(String, f32)>>> {
        self.validate_top_k(top_k)?;
        let sing = self.singularity.read().await;
        let mut out = Vec::with_capacity(queries.len());
        for query in queries {
            out.push(sing.find_similar(query, top_k));
        }
        Ok(out)
    }

    pub async fn probe_batch_cached(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Arc<[(String, f32)]>>> {
        self.validate_top_k(top_k)?;
        let sing = self.singularity.read().await;
        let mut out = Vec::with_capacity(queries.len());
        for query in queries {
            out.push(sing.find_similar_cached(query, top_k));
        }
        Ok(out)
    }

    pub async fn export_json(&self, path: &str) -> Result<()> {
        let payload = {
            let sing = self.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(),
                associations: sing.all_associations(),
            }
        };

        let data = serde_json::to_vec_pretty(&payload)?;
        fs::write(path, data).await?;
        Ok(())
    }

    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let bytes = fs::read(path).await?;
        let payload: ExportPayload = serde_json::from_slice(&bytes)?;

        if !merge {
            {
                let mut sing = self.singularity.write().await;
                sing.clear();
            }
            if let Some(ref persistence) = self.persistence {
                persistence.clear_all().await?;
            }
        }

        {
            let mut sing = self.singularity.write().await;
            let mut valid_associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(from, to, *strength) {
                    Ok(()) => valid_associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import_json"
                        );
                    }
                }
            }

            if let Some(ref persistence) = self.persistence {
                persistence.save_concepts(&payload.concepts).await?;
                persistence.save_associations(&valid_associations).await?;
            }
        }

        Ok(payload.concepts.len())
    }

    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let payload = {
            let sing = self.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(),
                associations: sing.all_associations(),
            }
        };

        let data = bincode::serialize(&payload).map_err(|e| {
            crate::error::MemoryError::Serialization(serde_json::Error::io(std::io::Error::other(
                e.to_string(),
            )))
        })?;
        fs::write(path, data).await?;
        Ok(())
    }

    pub async fn backup(&self, path: &str) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.backup(path).await?;
        }
        Ok(())
    }

    pub async fn restore(&self, path: &str) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.restore(path).await?;
            self.load_replace().await?;
        }
        Ok(())
    }

    pub async fn concept_history(
        &self,
        id: &str,
        limit: usize,
    ) -> Result<Vec<crate::persistence::ConceptVersion>> {
        if let Some(ref persistence) = self.persistence {
            return persistence.get_concept_history(id, limit).await;
        }
        Ok(Vec::new())
    }
}
