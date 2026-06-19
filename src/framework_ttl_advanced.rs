//! Advanced TTL policy and decay curve definitions for ChaoticSemanticFramework.

use serde::{Deserialize, Serialize};
pub use csm_memory::singularity::DecayCurve;

/// Policy for automatic TTL assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TtlPolicy {
    /// No automatic TTL (permanent unless explicitly set).
    None,
    /// Fixed TTL for all new concepts.
    Fixed(u64),
    /// Assign TTL based on metadata rules.
    MetadataRule(Vec<TtlRule>),
    /// Inherit TTL from associated concepts.
    Inherit,
}

impl Default for TtlPolicy {
    fn default() -> Self {
        Self::None
    }
}

/// Rule for metadata-based TTL assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtlRule {
    /// Metadata key to check.
    pub key: String,
    /// Metadata value to match (exact match).
    pub value: serde_json::Value,
    /// TTL in seconds to assign if matched.
    pub ttl_seconds: u64,
}

/// Advanced TTL and decay configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TtlConfig {
    /// Policy for automatic TTL assignment.
    pub policy: TtlPolicy,
    /// Curve for association strength decay.
    pub association_decay: DecayCurve,
    /// Interval for background cleanup (0 = disabled).
    pub cleanup_interval_seconds: u64,
    /// Whether to enable cascading purge (if A expires, its dependencies might too).
    pub cascading_purge: bool,
}
