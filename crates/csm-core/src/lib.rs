pub mod bundle;
pub mod bundle_simd;
pub mod encoder;
pub mod error;
pub mod hyperdim;
pub mod hyperdim_batch;
pub mod hyperdim_serde;
pub mod hyperdim_simd;
pub mod hyperdim_simd_bundle;
pub mod reservoir;
pub mod reservoir_chaotic;
pub mod reservoir_inertial;
pub mod reservoir_sparse;

#[cfg(test)]
mod hyperdim_tests;
#[cfg(test)]
mod reservoir_tests;

pub use error::{MemoryError, Result};
pub use hyperdim::{HVec10240, batch_cosine_similarity};
pub use bundle::BundleAccumulator;

pub mod prelude {
    pub use crate::error::{MemoryError, Result};
    pub use crate::hyperdim::HVec10240;
}
