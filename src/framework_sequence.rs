//! Reservoir sequence processing for the framework.

use crate::framework::ChaoticSemanticFramework;
use crate::framework_events_ce::ChaoticEvent;
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::HVec10240;
use csm_core_lib::reservoir_chaotic::ChaoticReservoir;
use std::sync::Arc;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// Process temporal sequence through reservoir.
    // Reservoir lock needed for sequence processing.
    #[instrument(err, skip(self, sequence))]
    pub async fn process_sequence(&self, sequence: &[Vec<f32>]) -> Result<HVec10240> {
        self.validate_sequence_length(sequence.len())?;

        let mut events = Vec::new();
        let mut reservoir_guard = self.reservoir.write().await;

        if reservoir_guard.is_none() {
            *reservoir_guard = Some(ChaoticReservoir::new_with_metrics(
                self.config.reservoir_input_size,
                self.config.reservoir_size,
                self.config.chaos_strength,
                Arc::clone(&self.metrics.reservoir_metrics),
            )?);
        }

        let r = reservoir_guard
            .as_mut()
            .ok_or(csm_core_lib::error::MemoryError::reservoir(
                "reservoir failed to initialize".to_string(),
            ))?;
        r.reset();

        for (step_idx, input) in sequence.iter().enumerate() {
            let out = r.step(input)?;
            events.push(ChaoticEvent::EchoComputed {
                input_dim: input.len(),
                state_norm: out.state_norm,
            });

            // Convergence detection: if change_norm is very small, we've hit an attractor basin.
            if out.change_norm < 1e-5 {
                events.push(ChaoticEvent::AttractorFired {
                    attractor_id: step_idx as u32,
                    basin_energy: out.change_norm,
                    reservoir_dim: self.config.reservoir_size,
                });
            }
        }
        let hv = r.to_hypervector()?;
        drop(reservoir_guard);

        for event in events {
            self.emit_chaotic_event(event).await;
        }

        Ok(hv)
    }
}
