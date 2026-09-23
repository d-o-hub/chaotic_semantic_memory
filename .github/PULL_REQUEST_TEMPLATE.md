## Summary

<!-- Brief description of changes -->

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)
- [ ] Performance improvement
- [ ] Test coverage improvement

## Related Issues

<!-- Link related issues: Closes #123, Fixes #456 -->

## Checklist

- [ ] `cargo check` passes
- [ ] `cargo test --all-features` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Source files remain at or below 500 LOC
- [ ] Benchmarks pass: `reservoir_step_50k < 100μs` (if performance-sensitive)
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (if user-facing change)

## Performance Evidence

<!-- Required when the PR title starts with `perf(`; leave blank otherwise.
     CI enforces this section for perf PRs via scripts/check-perf-pr-evidence.py
     (ADR-0095). A perf PR without evidence is not review-ready.
-->

- Baseline: `plans/evidence/bench/canonical.json` — benchmark id:
- Measurement artifact: Criterion output path or flamegraph link:
- Before / after for the affected benchmark:

## Additional Notes

<!-- Any additional context, screenshots, or concerns -->
