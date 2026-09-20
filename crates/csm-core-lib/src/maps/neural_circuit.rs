//! Cyclic Discrete Neural Circuit Map (CDNCM) — re-export of the owner crate.
//!
//! The implementation lives in `csm-chaos` (`maps::neural_circuit`), which owns
//! the chaotic maps (ADR-0094: one owner per surface). This module keeps
//! `csm_core_lib::maps::neural_circuit::NeuralCircuitMap` available for
//! downstream users of the `experimental-neural-circuit` feature; before the
//! dedup it held a second, unreachable copy of the same map.
//!
//! Based on Long et al., "A Cyclic discrete neural circuit map with hyperchaotic
//! dynamics and an application to adaptive DNA image encryption" (2026-07-23).
pub use csm_chaos::maps::neural_circuit::NeuralCircuitMap;
