#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod hashing;
pub mod maps;

pub use hashing::chaotic_lsh::ChaoticLsh;
#[cfg(feature = "experimental-coupled-lorenz")]
pub use maps::coupled_lorenz::CoupledLorenzMap;
pub use maps::hyperchaotic::Slhm2d;
#[cfg(feature = "experimental-ils3d")]
pub use maps::hyperchaotic_3d_ils::Ils3d;
pub use maps::hyperchaotic_chebyshev::ChebyshevLogistic2d;
