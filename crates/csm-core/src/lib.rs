#![allow(clippy::needless_range_loop, unused_imports)]
pub mod bundle;
pub mod bundle_simd;
pub mod encoder;
pub mod error;
pub mod hyperdim;
pub mod hyperdim_binary;
pub mod hyperdim_batch;
pub mod hyperdim_serde;
pub mod hyperdim_simd;
pub mod hyperdim_simd_bundle;
pub mod reservoir;
pub mod reservoir_chaotic;
pub mod reservoir_inertial;
pub mod reservoir_sparse;

pub use bundle::BundleAccumulator;
pub use error::{MemoryError, Result};
pub use hyperdim::{HVec10240, batch_cosine_similarity};
pub use hyperdim_binary::BHVec10240;

pub mod prelude {
    pub use crate::error::{MemoryError, Result};
    pub use crate::hyperdim::HVec10240;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;
    use crate::reservoir::Reservoir;

    mod hyperdim_tests {
        use super::*;
        include!("hyperdim_tests.rs");
    }
    mod reservoir_tests {
        use super::*;
        include!("reservoir_tests.rs");
    }
}
