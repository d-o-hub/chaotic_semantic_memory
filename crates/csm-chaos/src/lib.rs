#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod hashing;
pub mod maps;

pub use hashing::chaotic_lsh::ChaoticLsh;
pub use maps::hyperchaotic::Slhm2d;
pub use maps::hyperchaotic_chebyshev::ChebyshevLogistic2d;
