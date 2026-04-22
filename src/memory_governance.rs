//! Policy-Driven Memory Governance Layer
//!
//! Based on "MemArchitect: A Policy Driven Memory Governance Layer" (2026).
//! This module provides a governance layer that decouples memory lifecycle
//! management from core retrieval logic. It enforces rule-based policies such
//! as memory decay, conflict resolution, and privacy controls, preventing
//! "zombie memories" from contaminating the context window.

use crate::metadata_filter::MetadataFilter;
use crate::singularity::Concept;
use serde::{Deserialize, Serialize};

/// Defines the decay policy for memory concepts over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecayPolicy {
    /// No decay (default)
    None,
    /// Exponential decay based on time since `created_at` or `modified_at`
    Exponential { half_life_secs: u64 },
    /// Fixed expiration time (already supported natively by `expires_at` but explicit here)
    FixedExpiration { ttl_secs: u64 },
}

/// Defines conflict resolution policies for concepts with the same semantic triple or high similarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionPolicy {
    /// Keep the most recent concept
    KeepNewest,
    /// Keep the oldest concept
    KeepOldest,
    /// Merge metadata of conflicting concepts
    Merge,
}

/// Defines privacy controls to restrict retrieval based on metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyControl {
    /// Metadata filter that must evaluate to true for a concept to be visible
    pub visibility_filter: Option<MetadataFilter>,
    /// Metadata keys that should be stripped before returning the concept
    pub redacted_keys: Vec<String>,
}

/// A policy set defining governance rules for memory management.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernancePolicy {
    pub decay: Option<DecayPolicy>,
    pub conflict_resolution: Option<ConflictResolutionPolicy>,
    pub privacy: Option<PrivacyControl>,
}

impl GovernancePolicy {
    /// Applies governance policies to a retrieved concept.
    /// Returns `None` if the concept should be filtered out (e.g., decayed to zero or private).
    pub fn apply(&self, concept: Concept, current_time_secs: u64) -> Option<Concept> {
        let mut managed_concept = concept;

        // 1. Apply Privacy Controls
        if let Some(privacy) = &self.privacy {
            if let Some(filter) = &privacy.visibility_filter {
                if !filter.matches(&managed_concept.metadata) {
                    return None;
                }
            }
            // Strip redacted keys
            for key in &privacy.redacted_keys {
                managed_concept.metadata.remove(key);
            }
        }

        // 2. Apply Decay Policies
        if let Some(decay) = &self.decay {
            match decay {
                DecayPolicy::Exponential { half_life_secs } => {
                    let age = current_time_secs.saturating_sub(managed_concept.modified_at);
                    // Standard exponential decay: N(t) = N_0 * (1/2)^(t/t_half)
                    let decay_factor = 0.5f32.powf(age as f32 / *half_life_secs as f32);

                    // If decayed beyond a threshold (e.g., 0.05), consider it a "zombie memory"
                    if decay_factor < 0.05 {
                        return None;
                    }

                    // Optional: store decay factor in metadata for downstream confidence weighting
                    managed_concept.metadata.insert(
                        "governance_decay_factor".to_string(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(decay_factor as f64).unwrap(),
                        ),
                    );
                }
                DecayPolicy::FixedExpiration { ttl_secs } => {
                    let age = current_time_secs.saturating_sub(managed_concept.created_at);
                    if age >= *ttl_secs {
                        return None;
                    }
                }
                DecayPolicy::None => {}
            }
        }

        Some(managed_concept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singularity::ConceptBuilder;

    #[test]
    fn test_privacy_filtering() {
        let policy = GovernancePolicy {
            privacy: Some(PrivacyControl {
                visibility_filter: Some(MetadataFilter::eq("tenant_id", "tenant-1")),
                redacted_keys: vec!["ssn".to_string()],
            }),
            ..Default::default()
        };

        let concept = ConceptBuilder::new("c1")
            .with_metadata("tenant_id", "tenant-1")
            .with_metadata("ssn", "123-456-7890")
            .with_metadata("public_info", "hello")
            .build()
            .unwrap();

        let applied = policy.apply(concept, 100).unwrap();
        assert_eq!(
            applied.metadata.get("tenant_id").unwrap().as_str().unwrap(),
            "tenant-1"
        );
        assert_eq!(
            applied
                .metadata
                .get("public_info")
                .unwrap()
                .as_str()
                .unwrap(),
            "hello"
        );
        assert!(
            !applied.metadata.contains_key("ssn"),
            "SSN should be redacted"
        );

        let concept_wrong_tenant = ConceptBuilder::new("c2")
            .with_metadata("tenant_id", "tenant-2")
            .build()
            .unwrap();

        let applied_wrong = policy.apply(concept_wrong_tenant, 100);
        assert!(
            applied_wrong.is_none(),
            "Concept should be filtered due to visibility constraint"
        );
    }

    #[test]
    fn test_exponential_decay() {
        let policy = GovernancePolicy {
            decay: Some(DecayPolicy::Exponential {
                half_life_secs: 100,
            }),
            ..Default::default()
        };

        let concept = ConceptBuilder::new("c1").build().unwrap(); // Assume modified_at is ~current time

        // Current time is same as creation/modification time -> decay_factor = 1.0
        let applied_now = policy.apply(concept.clone(), concept.modified_at).unwrap();
        let decay_now = applied_now
            .metadata
            .get("governance_decay_factor")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(decay_now, 1.0);

        // 100 seconds later (1 half-life) -> decay_factor = 0.5
        let applied_1_hl = policy
            .apply(concept.clone(), concept.modified_at + 100)
            .unwrap();
        let decay_1_hl = applied_1_hl
            .metadata
            .get("governance_decay_factor")
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((decay_1_hl - 0.5).abs() < 0.001);

        // 500 seconds later (5 half-lives) -> decay_factor = (0.5)^5 = 0.03125 (Below threshold of 0.05)
        let applied_zombie = policy.apply(concept.clone(), concept.modified_at + 500);
        assert!(
            applied_zombie.is_none(),
            "Memory should be considered zombie and filtered"
        );
    }
}
