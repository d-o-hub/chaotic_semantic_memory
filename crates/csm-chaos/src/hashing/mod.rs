pub mod chaotic_lsh;
#[cfg(feature = "experimental-voronoi-lsh")]
pub mod voronoi_lsh;

/// Largest input dimension either LSH projector will accept.
///
/// The bound is a memory bound, not a "typical embedding" guess: both
/// projectors materialise a projection table proportional to `input_dim`.
///
/// | projector | bytes per instance |
/// |---|---:|
/// | [`chaotic_lsh::ChaoticLsh`] | `10240 * input_dim * 4` |
/// | [`voronoi_lsh::VoronoiLsh`] | `10240 * 2 * input_dim * 4` |
///
/// At this cap that is 160 MiB per [`chaotic_lsh::ChaoticLsh`] and 320 MiB per
/// [`voronoi_lsh::VoronoiLsh`] (the latter only exists behind
/// `experimental-voronoi-lsh`). The largest dimension used anywhere in this
/// repository is 1536 (the chaotic-hashing benchmark), and the next common
/// embedding width is 3072, so 4096 leaves room for both while refusing the
/// GiB-scale allocations an unbounded `usize` allowed.
///
/// `input_dim` is a shape invariant, not a size hint: a projector built for a
/// different dimension returns an all-zero hypervector for every query. Use
/// [`chaotic_lsh::ChaoticLsh::try_new`] (or
/// [`voronoi_lsh::VoronoiLsh::try_new`]) when the dimension comes from an
/// untrusted caller; it rejects an oversized request instead of substituting a
/// different shape.
pub const MAX_INPUT_DIM: usize = 4096;

/// Error returned when an input dimension is outside
/// `1..=`[`MAX_INPUT_DIM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputDimensionError {
    /// The dimension the caller asked for.
    pub requested: usize,
    /// The dimension the projector can represent.
    pub max: usize,
}

impl core::fmt::Display for InputDimensionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "input_dim {} is outside the supported range 1..={}",
            self.requested, self.max
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InputDimensionError {}

/// Validate a caller-supplied input dimension.
///
/// Rejects zero as well as values above [`MAX_INPUT_DIM`]: both are shape
/// errors, and a zero-width projector can only ever return all-zero codes.
pub const fn validate_input_dim(requested: usize) -> Result<usize, InputDimensionError> {
    if requested == 0 || requested > MAX_INPUT_DIM {
        return Err(InputDimensionError {
            requested,
            max: MAX_INPUT_DIM,
        });
    }
    Ok(requested)
}

/// Clamp an input dimension for the infallible constructors.
///
/// Kept separate from [`validate_input_dim`] so the substitution can be tested
/// without allocating the clamped table.
#[must_use]
pub(crate) fn clamped_input_dim(requested: usize) -> usize {
    requested.clamp(1, MAX_INPUT_DIM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_the_supported_range() {
        assert_eq!(validate_input_dim(1).unwrap(), 1);
        assert_eq!(validate_input_dim(1536).unwrap(), 1536);
        assert_eq!(validate_input_dim(MAX_INPUT_DIM).unwrap(), MAX_INPUT_DIM);
    }

    #[test]
    fn validate_rejects_zero_and_oversized_dimensions() {
        let zero = validate_input_dim(0).unwrap_err();
        assert_eq!(zero.requested, 0);
        assert_eq!(zero.max, MAX_INPUT_DIM);
        assert!(alloc::format!("{zero}").contains("1..="));

        let oversized = validate_input_dim(MAX_INPUT_DIM + 1).unwrap_err();
        assert_eq!(oversized.requested, MAX_INPUT_DIM + 1);
        assert_eq!(oversized.max, MAX_INPUT_DIM);
    }

    #[test]
    fn clamp_is_bounded_at_both_ends() {
        assert_eq!(clamped_input_dim(0), 1);
        assert_eq!(clamped_input_dim(1536), 1536);
        assert_eq!(clamped_input_dim(MAX_INPUT_DIM), MAX_INPUT_DIM);
        assert_eq!(clamped_input_dim(usize::MAX), MAX_INPUT_DIM);
    }
}
