//! Association decay: reinforcement and pruning for weighted forgetting (ADR-0025).

use csm_core::error::{MemoryError, Result};
use csm_core::hyperdim::Hypervector;

use crate::singularity::{DecayCurve, Singularity, unix_now_secs};

impl<H: Hypervector + 'static> Singularity<H> {
    /// Reinforce an association by resetting its `created_at` timestamp to now.
    /// This effectively refreshes the association so decay starts over.
    pub fn reinforce_association(&mut self, ns: &str, from: &str, to: &str) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        let neighbors =
            ns_state
                .associations
                .get_mut(from)
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "Association".to_string(),
                    id: format!("{from} -> {to}"),
                })?;
        let entry = neighbors.get_mut(to).ok_or_else(|| MemoryError::NotFound {
            entity: "Association".to_string(),
            id: format!("{from} -> {to}"),
        })?;
        entry.1 = unix_now_secs();
        Ok(())
    }

    /// Prune associations whose decayed strength falls below `threshold`.
    /// Returns the number of associations removed.
    pub fn prune_decayed_associations(
        &mut self,
        ns: &str,
        curve: DecayCurve,
        threshold: f32,
    ) -> usize {
        let now = unix_now_secs();
        let ns_state = self.get_namespace_mut(ns);
        let mut removed = 0usize;
        for neighbors in ns_state.associations.values_mut() {
            let before = neighbors.len();
            neighbors.retain(|_, (strength, created_at)| {
                let elapsed = now.saturating_sub(*created_at);
                curve.apply(*strength, elapsed) >= threshold
            });
            removed += before - neighbors.len();
        }
        // Remove empty neighbor maps
        ns_state.associations.retain(|_, v| !v.is_empty());
        removed
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::ConceptBuilder;
    use crate::singularity::SingularityConfig;
    use csm_core::HVec10240;

    fn make_singularity() -> Singularity<HVec10240> {
        Singularity::new(SingularityConfig::default())
    }

    fn inject(sing: &mut Singularity<HVec10240>, ns: &str, id: &str) {
        let concept = ConceptBuilder::new(id)
            .with_vector(HVec10240::random())
            .build()
            .unwrap();
        sing.inject(ns, concept).unwrap();
    }

    #[test]
    fn reinforce_resets_created_at() {
        let mut sing = make_singularity();
        let ns = "_default";
        inject(&mut sing, ns, "a");
        inject(&mut sing, ns, "b");
        sing.associate(ns, "a", "b", 0.8).unwrap();

        // Reinforcing should succeed
        assert!(sing.reinforce_association(ns, "a", "b").is_ok());
    }

    #[test]
    fn reinforce_nonexistent_returns_error() {
        let mut sing = make_singularity();
        let ns = "_default";
        inject(&mut sing, ns, "a");

        assert!(sing.reinforce_association(ns, "a", "b").is_err());
        assert!(sing.reinforce_association(ns, "x", "y").is_err());
    }

    #[test]
    fn prune_removes_decayed_associations() {
        let mut sing = make_singularity();
        let ns = "_default";
        inject(&mut sing, ns, "a");
        inject(&mut sing, ns, "b");
        inject(&mut sing, ns, "c");
        sing.associate(ns, "a", "b", 0.9).unwrap();
        sing.associate(ns, "a", "c", 0.9).unwrap();

        // With no decay, nothing should be pruned
        let removed = sing.prune_decayed_associations(ns, DecayCurve::None, 0.5);
        assert_eq!(removed, 0);

        // With a step decay that drops 1.0 immediately, everything should be pruned
        let curve = DecayCurve::Step {
            threshold_seconds: 0,
            drop: 1.0,
        };
        let removed = sing.prune_decayed_associations(ns, curve, 0.5);
        assert_eq!(removed, 2);
        assert!(sing.get_associations(ns, "a").is_empty());
    }
}
