//! Association decay: reinforcement and pruning for weighted forgetting (ADR-0025).

use csm_core_lib::error::{MemoryError, Result};
use csm_core_lib::hyperdim::Hypervector;

use crate::singularity::{DecayCurve, Singularity, unix_now_secs};

impl<H: Hypervector + 'static> Singularity<H> {
    /// Reinforce an association by resetting its `created_at` timestamp to now.
    /// This effectively refreshes the association so decay starts over.
    pub fn reinforce_association(&mut self, ns: &str, from: &str, to: &str) -> Result<()> {
        let ns_state = self.ensure_namespace(ns)?;
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
    ///
    /// `threshold` must be finite and within `[0.0, 1.0]`; invalid values are
    /// rejected with `MemoryError::InvalidInput` before any state is touched.
    /// A `NaN` threshold would otherwise silently prune every association,
    /// since every `decayed >= NaN` comparison is false.
    ///
    /// Input validation runs before the namespace lookup, so an invalid
    /// threshold errors even for a missing namespace. Missing namespaces are
    /// otherwise a no-op (returns 0) so prune never creates an empty namespace
    /// solely to count removals.
    pub fn prune_decayed_associations(
        &mut self,
        ns: &str,
        curve: DecayCurve,
        threshold: f32,
    ) -> Result<usize> {
        validate_prune_threshold(threshold)?;
        let now = unix_now_secs();
        let Some(ns_state) = self.namespaces.get_mut(ns) else {
            return Ok(0);
        };
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
        Ok(removed)
    }
}

/// Validate a decay-prune threshold: finite and within `[0.0, 1.0]`.
///
/// Mirrors the framework-level `validate_prune_threshold` (src/framework_validation.rs)
/// so the raw `Singularity` API rejects the same malicious inputs that would
/// otherwise silently mass-prune associations.
fn validate_prune_threshold(threshold: f32) -> Result<()> {
    if !threshold.is_finite() {
        return Err(MemoryError::InvalidInput {
            field: "threshold".to_string(),
            reason: "prune threshold must be finite".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&threshold) {
        return Err(MemoryError::InvalidInput {
            field: "threshold".to_string(),
            reason: format!("prune threshold must be in [0.0, 1.0], got {threshold}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::ConceptBuilder;
    use crate::singularity::SingularityConfig;
    use csm_core_lib::HVec10240;

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
        let removed = sing
            .prune_decayed_associations(ns, DecayCurve::None, 0.5)
            .unwrap();
        assert_eq!(removed, 0);

        // With a step decay that drops 1.0 immediately, everything should be pruned
        let curve = DecayCurve::Step {
            threshold_seconds: 0,
            drop: 1.0,
        };
        let removed = sing.prune_decayed_associations(ns, curve, 0.5).unwrap();
        assert_eq!(removed, 2);
        assert!(sing.get_associations(ns, "a").is_empty());
    }

    #[test]
    fn prune_rejects_invalid_thresholds() {
        let mut sing = make_singularity();
        let ns = "_default";

        for bad in [-0.1, 1.1, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let err = sing
                .prune_decayed_associations(ns, DecayCurve::None, bad)
                .unwrap_err();
            let MemoryError::InvalidInput { field, .. } = err else {
                panic!("expected InvalidInput, got: {err:?}");
            };
            assert_eq!(field, "threshold");
        }

        // Inclusive range edges are valid no-ops on an empty namespace
        assert_eq!(
            sing.prune_decayed_associations(ns, DecayCurve::None, 0.0)
                .unwrap(),
            0
        );
        assert_eq!(
            sing.prune_decayed_associations(ns, DecayCurve::None, 1.0)
                .unwrap(),
            0
        );
    }
}
