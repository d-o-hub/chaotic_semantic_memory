# GOAP Plan: Real-Life CLI Edge Case Examples

## Current State
- CLI commands implemented: inject, probe, associate, export, import, completions
- Edge case coverage exists in tests/ but no real-life CLI examples
- Documentation has basic examples but lacks edge case demonstrations
- All validation gates passing, production-ready

## Target State
- Comprehensive CLI examples for all edge cases in `examples/cli/`
- Each example demonstrates one edge case with real-world context
- Examples are executable and validate the edge case behavior
- Handoff artifacts document example patterns

## Actions

### Phase 1: Planning & Infrastructure
- **name**: create_example_infrastructure
  - preconditions: [cli_commands_implemented]
  - effects: [example_infrastructure_ready]
  - cost: 2
  - Create `examples/cli/` directory structure
  - Create shared utilities for example scripts

### Phase 2: Core Edge Case Examples (Parallel Groups)

#### Group A: Hypervector & Validation Edge Cases
- **name**: create_hypervector_examples
  - preconditions: [example_infrastructure_ready]
  - effects: [hypervector_examples_complete]
  - cost: 4
  - Examples:
    - `01_empty_concept_id.sh` - Empty ID rejection
    - `02_oversized_concept_id.sh` - 257+ byte ID rejection  
    - `03_invalid_vector_format.sh` - Malformed vector data
    - `04_similarity_bounds.sh` - Demonstrate [-1, 1] bounds

#### Group B: Reservoir & Sequence Edge Cases
- **name**: create_reservoir_examples
  - preconditions: [example_infrastructure_ready]
  - effects: [reservoir_examples_complete]
  - cost: 4
  - Examples:
    - `05_empty_sequence.sh` - Empty sequence returns zero vector
    - `06_spectral_radius_bounds.sh` - Invalid radius rejection
    - `07_reservoir_size_limits.sh` - Size < DIMENSION error
    - `08_sequence_temporal.sh` - Long sequence processing

#### Group C: Association & Framework Edge Cases
- **name**: create_framework_examples
  - preconditions: [example_infrastructure_ready]
  - effects: [framework_examples_complete]
  - cost: 4
  - Examples:
    - `09_negative_strength.sh` - Negative association rejection
    - `10_top_k_limits.sh` - Zero and excessive top_k handling
    - `11_self_association.sh` - Self-association warning
    - `12_metadata_limits.sh` - Metadata size enforcement

#### Group D: Persistence & Data Edge Cases
- **name**: create_persistence_examples
  - preconditions: [example_infrastructure_ready]
  - effects: [persistence_examples_complete]
  - cost: 4
  - Examples:
    - `13_import_missing_file.sh` - Non-existent file error
    - `14_duplicate_concept.sh` - Update vs create behavior
    - `15_export_empty_db.sh` - Empty database warning
    - `16_batch_operations.sh` - Empty and large batches

### Phase 3: Integration & Validation
- **name**: integrate_all_examples
  - preconditions: [hypervector_examples_complete, reservoir_examples_complete, framework_examples_complete, persistence_examples_complete]
  - effects: [all_examples_integrated]
  - cost: 3
  - Create master run script
  - Link examples to documentation
  - Generate handoff artifact

- **name**: validate_cli_examples
  - preconditions: [all_examples_integrated]
  - effects: [examples_validated]
  - cost: 2
  - Run all examples against CLI
  - Verify expected outputs
  - Update GOAP_STATE

## Handoff Contracts

1. **A->All**: Validation patterns for malformed input handling
2. **B->All**: Configuration boundaries and chaos parameter patterns
3. **C->All**: Association and query limit conventions
4. **D->All**: File I/O error handling and batch operation patterns

## Success Criteria
- All 16 example scripts exist and are executable
- Each script exits with expected code (0 for success, non-zero for errors)
- Examples demonstrate real-world contexts (e.g., "Importing corrupted data file")
- Documentation references examples
