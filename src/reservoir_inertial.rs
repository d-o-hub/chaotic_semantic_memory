//! Inertial reservoir extension (ADR-0064).
//!
//! Second-order momentum dynamics for improved temporal memory retention.

use crate::error::{MemoryError, Result};
use crate::reservoir::Reservoir;

impl Reservoir {
    /// Set the inertial coefficient (beta) for second-order dynamics.
    ///
    /// The inertial term adds momentum from previous state changes:
    /// `state[t] = ... + β·(state[t-1] - state[t-2])`
    ///
    /// Valid range: β ∈ [0.0, 0.5]
    /// - β = 0.0: No inertia (backward-compatible, original behavior)
    /// - β > 0.0: Increased memory retention through momentum
    ///
    /// Paper: Zhao et al., "Inertial ESN", Neurocomputing Apr 2026
    pub fn with_beta(mut self, beta: f32) -> Result<Self> {
        if !(0.0..=0.5).contains(&beta) {
            return Err(MemoryError::reservoir(format!(
                "Beta must be in [0.0, 0.5], got {}",
                beta
            )));
        }
        self.beta = beta;
        Ok(self)
    }

    /// Get current beta coefficient.
    pub const fn beta(&self) -> f32 {
        self.beta
    }
}
