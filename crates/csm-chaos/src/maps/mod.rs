#[cfg(feature = "experimental-coupled-lorenz")]
pub mod coupled_lorenz;
pub mod hyperchaotic;
#[cfg(feature = "experimental-ils3d")]
pub mod hyperchaotic_3d_ils;
pub mod hyperchaotic_chebyshev;
#[cfg(feature = "experimental-neural-circuit")]
pub mod neural_circuit;
