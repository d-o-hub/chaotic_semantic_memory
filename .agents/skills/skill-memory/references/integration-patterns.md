# Integration Patterns by Skill

This document provides concrete integration patterns for each skill type.

## Table of Contents

1. [adr-creation](#adr-creation)
2. [debugging-reservoir](#debugging-reservoir)
3. [rust-development](#rust-development)
4. [testing-validation](#testing-validation)
5. [goap-planning](#goap-planning)

---

## adr-creation

### Pattern: Decision Precedent Lookup

**Use Case:** When creating a new ADR, suggest related past decisions.

```rust
use skill_memory::SkillMemory;

pub struct ADRMemoryContext {
    memory: SkillMemory,
}

impl ADRMemoryContext {
    pub async fn initialize() -> Result<Self, MemoryError> {
        Ok(Self {
            memory: SkillMemory::initialize("adr-creation").await?,
        })
    }

    /// Remember a new architectural decision
    pub async fn remember_decision(
        &self,
        adr: &ADR,
    ) -> Result<String, MemoryError> {
        let context = format!(
            "ADR-{:04d}: {}. Context: {}",
            adr.number, adr.title, adr.context
        );

        let result = format!(
            "Decision: {}. Consequences: {:?}",
            adr.decision, adr.consequences
        );

        let concept_id = self.memory.remember(
            "architectural_decision",
            &context,
            &result
        ).await?;

        // Associate with related ADRs
        for related_num in &adr.related {
            let related_id = format!("adr::{}", related_num);
            self.memory.associate(
                &concept_id,
                &related_id,
                0.8
            ).await?;
        }

        Ok(concept_id)
    }

    /// Find related past decisions
    pub async fn find_related_decisions(
        &self,
        topic: &str,
    ) -> Result<Vec<DecisionReference>, MemoryError> {
        let memories = self.memory.recall(topic, 0.75, 5).await?;

        let mut decisions = Vec::new();
        for entry in memories {
            // Parse ADR number from concept ID
            if let Some(adr_num) = self.extract_adr_number(&entry.id) {
                decisions.push(DecisionReference {
                    adr_number: adr_num,
                    title: entry.operation.clone(),
                    similarity: entry.similarity,
                    decision_summary: entry.result.clone(),
                });
            }
        }

        Ok(decisions)
    }

    /// Get decisions that led to a specific consequence
    pub async fn find_decisions_by_consequence(
        &self,
        consequence: &str,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // Query for decisions mentioning this consequence
        self.memory.recall(
            &format!("architectural decision {}", consequence),
            0.6,
            10
        ).await
    }

    fn extract_adr_number(&self,
        concept_id: &str
    ) -> Option<u32> {
        // Parse from ID format: "skill::adr-creation::architectural_decision::{timestamp}"
        // Extract from metadata or parse from operation field
        None  // Implementation details
    }
}

pub struct DecisionReference {
    pub adr_number: u32,
    pub title: String,
    pub similarity: f32,
    pub decision_summary: String,
}

pub struct ADR {
    pub number: u32,
    pub title: String,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub related: Vec<u32>,
}
```

### Usage Example

```rust
async fn create_adr(
    context: &str,
    proposed_decision: &str,
) -> Result<ADR, Box<dyn std::error::Error>> {
    let memory = ADRMemoryContext::initialize().await?;

    // Find related past decisions
    let related = memory.find_related_decisions(context).await?;

    println!("Related decisions found:");
    for decision in &related {
        println!(
            "  ADR-{:04d}: {} (similarity: {:.2})",
            decision.adr_number,
            decision.title,
            decision.similarity
        );
    }

    // Create ADR with related references
    let adr = ADR {
        number: get_next_adr_number(),
        title: proposed_decision.to_string(),
        context: context.to_string(),
        decision: proposed_decision.to_string(),
        consequences: vec![],
        related: related.iter().map(|d| d.adr_number).collect(),
    };

    // Remember this decision
    memory.remember_decision(&adr).await?;

    Ok(adr)
}
```

---

## debugging-reservoir

### Pattern: Error-Solution Knowledge Graph

**Use Case:** Build a graph of symptoms → causes → solutions.

```rust
use skill_memory::SkillMemory;

pub struct ReservoirDebugMemory {
    memory: SkillMemory,
}

impl ReservoirDebugMemory {
    pub async fn initialize() -> Result<Self, MemoryError> {
        Ok(Self {
            memory: SkillMemory::initialize("debugging-reservoir").await?,
        })
    }

    /// Remember an error pattern and its resolution
    pub async fn remember_error_resolution(
        &self,
        symptom: &str,
        cause: &str,
        solution: &str,
        effectiveness: f32,  // 0.0-1.0
    ) -> Result<String, MemoryError> {
        // Store the error-solution pair
        let context = format!("symptom: {}", symptom);
        let result = format!("cause: {}, solution: {}", cause, solution);

        let error_id = self.memory.remember(
            "reservoir_error_resolution",
            &context,
            &result
        ).await?;

        // Create typed associations
        let cause_id = format!("reservoir::cause::{}",
            cause.replace(" ", "_"));
        let solution_id = format!("reservoir::solution::{}",
            solution.replace(" ", "_"));

        // Error → Cause (high confidence)
        self.memory.associate(
            &error_id, &cause_id, 0.95
        ).await?;

        // Cause → Solution (based on effectiveness)
        self.memory.associate(
            &cause_id, &solution_id, effectiveness
        ).await?;

        // Solution → Error (bidirectional, for finding similar errors)
        self.memory.associate(
            &solution_id, &error_id, effectiveness
        ).await?;

        Ok(error_id)
    }

    /// Find solutions for a symptom
    pub async fn find_solutions(
        &self,
        symptom: &str,
    ) -> Result<Vec<Solution>, MemoryError> {
        // Find similar error patterns
        let similar_errors = self.memory.recall(symptom, 0.6, 5).await?;

        let mut solutions = Vec::new();
        for error in similar_errors {
            // Get causes for this error
            let causes = self.memory.related(&error.id, 0.8
            ).await?;

            for (cause, cause_strength) in causes {
                // Get solutions for this cause
                let cause_solutions = self.memory.related(
                    &cause.id, 0.7
                ).await?;

                for (solution, solution_strength) in cause_solutions {
                    // Combine strengths
                    let combined = cause_strength * solution_strength;

                    solutions.push(Solution {
                        description: solution.result.clone(),
                        confidence: combined,
                        source_error: error.id.clone(),
                    });
                }
            }
        }

        // Sort by confidence
        solutions.sort_by(|a, b|
            b.confidence.partial_cmp(&a.confidence).unwrap()
        );

        Ok(solutions)
    }

    /// Get common causes for a reservoir configuration
    pub async fn get_common_causes(
        &self,
        config: &ReservoirConfig,
    ) -> Result<Vec<CauseFrequency>, MemoryError> {
        // Query for errors with similar configs
        let query = format!(
            "reservoir error size:{} radius:{:?}",
            config.size, config.spectral_radius
        );

        let errors = self.memory.recall(&query, 0.5, 20).await?;

        // Aggregate causes
        let mut cause_counts: HashMap<String, usize> = HashMap::new();
        for error in errors {
            let causes = self.memory.related(&error.id, 0.8
            ).await?;
            for (cause, _) in causes {
                *cause_counts.entry(cause.id.clone()).or_insert(0) += 1;
            }
        }

        // Convert to frequencies
        let mut frequencies: Vec<_> = cause_counts
            .into_iter()
            .map(|(cause, count)| CauseFrequency {
                cause,
                occurrence_count: count,
            })
            .collect();

        frequencies.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

        Ok(frequencies)
    }
}

pub struct Solution {
    pub description: String,
    pub confidence: f32,
    pub source_error: String,
}

pub struct CauseFrequency {
    pub cause: String,
    pub occurrence_count: usize,
}

pub struct ReservoirConfig {
    pub size: usize,
    pub spectral_radius: f32,
}
```

### Usage Example

```rust
async fn diagnose_reservoir_issue(
    symptom: &str,
    config: &ReservoirConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let memory = ReservoirDebugMemory::initialize().await?;

    // Find solutions for this symptom
    let solutions = memory.find_solutions(symptom).await?;

    println!("Potential solutions for '{}':", symptom);
    for (i, solution) in solutions.iter().take(3).enumerate() {
        println!(
            "  {}. {} (confidence: {:.2})",
            i + 1,
            solution.description,
            solution.confidence
        );
    }

    // Get common causes for this config
    let common_causes = memory.get_common_causes(config).await?;

    println!("\nCommon issues with this configuration:");
    for cause in common_causes.iter().take(3) {
        println!("  - {} ({} occurrences)",
            cause.cause, cause.occurrence_count);
    }

    // If we fix it, remember the resolution
    let fix = apply_best_solution(&solutions).await?;
    memory.remember_error_resolution(
        symptom,
        &fix.cause,
        &fix.solution,
        fix.effectiveness,
    ).await?;

    Ok(())
}
```

---

## rust-development

### Pattern: Code Transformation Library

**Use Case:** Remember and recall refactoring patterns.

```rust
use skill_memory::SkillMemory;

pub struct CodePatternMemory {
    memory: SkillMemory,
}

impl CodePatternMemory {
    pub async fn initialize() -> Result<Self, MemoryError> {
        Ok(Self {
            memory: SkillMemory::initialize("rust-development").await?,
        })
    }

    /// Remember a code transformation
    pub async fn remember_transformation(
        &self,
        file_path: &str,
        pattern_type: TransformationType,
        before: &str,
        after: &str,
        metrics: &TransformationMetrics,
    ) -> Result<String, MemoryError> {
        let context = format!(
            "file: {}, type: {:?}, loc_before: {}, complexity_before: {}",
            file_path, pattern_type, metrics.loc_before,
            metrics.cyclomatic_before
        );

        let result = format!(
            "loc_after: {}, complexity_after: {}, tests_pass: {}, \
             time_ms: {}",
            metrics.loc_after, metrics.cyclomatic_after,
            metrics.tests_pass, metrics.duration_ms
        );

        let transform_id = self.memory.remember(
            "code_transformation",
            &context,
            &result
        ).await?;

        // Store the actual code in metadata via framework
        let vector = self.memory.text_to_hypervector(
            &format!("{:?} {}", pattern_type, before)
        ).await?;

        self.memory.framework().inject_concept_with_metadata(
            &format!("{}::code", transform_id),
            vector,
            hashmap! {
                "before" => before,
                "after" => after,
                "pattern_type" => format!("{:?}", pattern_type),
            }
        ).await?;

        // Associate transformation with pattern type
        let pattern_concept = format!("pattern::{:?}", pattern_type);
        self.memory.associate(
            &transform_id, &pattern_concept, 1.0
        ).await?;

        Ok(transform_id)
    }

    /// Find similar transformations
    pub async fn find_similar_transformations(
        &self,
        code_snippet: &str,
        pattern_type: Option<TransformationType>,
    ) -> Result<Vec<TransformationExample>, MemoryError> {
        // Build query based on pattern type
        let query = match pattern_type {
            Some(pt) => format!("{:?} {}", pt, code_snippet),
            None => code_snippet.to_string(),
        };

        let memories = self.memory.recall(&query, 0.65, 10
        ).await?;

        let mut examples = Vec::new();
        for entry in memories {
            // Get the code details
            let code_concept_id = format!("{}::code", entry.id);
            if let Some(concept) = self.memory.framework()
                .get_concept(&code_concept_id).await? {

                examples.push(TransformationExample {
                    pattern_type: concept.metadata
                        .get("pattern_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    before: concept.metadata
                        .get("before")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    after: concept.metadata
                        .get("after")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    similarity: entry.similarity,
                    success: entry.result.contains("tests_pass: true"),
                });
            }
        }

        Ok(examples)
    }

    /// Get success rate for a pattern type
    pub async fn get_pattern_success_rate(
        &self,
        pattern_type: TransformationType,
    ) -> Result<f32, MemoryError> {
        // Find all transformations of this type
        let pattern_concept = format!("pattern::{:?}", pattern_type);
        let related = self.memory.framework()
            .get_associations(&pattern_concept).await?;

        if related.is_empty() {
            return Ok(0.0);
        }

        let mut success_count = 0;
        for (transform_id, _) in related {
            if let Some(concept) = self.memory.framework()
                .get_concept(&transform_id).await? {
                if concept.metadata.get("result")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("tests_pass: true"))
                    .unwrap_or(false) {
                    success_count += 1;
                }
            }
        }

        Ok(success_count as f32 / related.len() as f32)
    }
}

#[derive(Debug, Clone)]
pub enum TransformationType {
    MatchToTrait,
    ExtractMethod,
    InlineVariable,
    ConvertToIterator,
    SimplifyMatch,
}

pub struct TransformationMetrics {
    pub loc_before: usize,
    pub loc_after: usize,
    pub cyclomatic_before: usize,
    pub cyclomatic_after: usize,
    pub tests_pass: bool,
    pub duration_ms: u64,
}

pub struct TransformationExample {
    pub pattern_type: String,
    pub before: String,
    pub after: String,
    pub similarity: f32,
    pub success: bool,
}
```

### Usage Example

```rust
async fn refactor_match_to_trait(
    file_path: &str,
    code: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let memory = CodePatternMemory::initialize().await?;

    // Find similar past transformations
    let examples = memory.find_similar_transformations(
        code,
        Some(TransformationType::MatchToTrait)
    ).await?;

    if let Some(best) = examples.first() {
        println!("Found similar transformation (similarity: {:.2}):",
            best.similarity);
        println!("Before:\n{}", best.before);
        println!("\nAfter:\n{}", best.after);

        if best.success {
            println!("This pattern has been tested successfully!");
        }
    }

    // Apply transformation
    let transformed = apply_transformation(code).await?;

    // Remember this transformation
    let metrics = measure_transformation(code, &transformed).await?;
    memory.remember_transformation(
        file_path,
        TransformationType::MatchToTrait,
        code,
        &transformed,
        &metrics,
    ).await?;

    Ok(transformed)
}
```

---

## testing-validation

### Pattern: Flaky Test Detection

**Use Case:** Remember test failure patterns and solutions.

```rust
use skill_memory::SkillMemory;

pub struct TestMemory {
    memory: SkillMemory,
}

impl TestMemory {
    pub async fn initialize() -> Result<Self, MemoryError> {
        Ok(Self {
            memory: SkillMemory::initialize("testing-validation").await?,
        })
    }

    /// Remember a test failure
    pub async fn remember_failure(
        &self,
        test_name: &str,
        error: &str,
        stack_trace: &str,
        solution: Option<&str>,
    ) -> Result<String, MemoryError> {
        let context = format!("test: {}, error: {}", test_name, error);
        let result = match solution {
            Some(s) => format!("solution: {}", s),
            None => "unresolved".to_string(),
        };

        let failure_id = self.memory.remember(
            "test_failure",
            &context,
            &result
        ).await?;

        // Associate with test
        let test_concept = format!("test::{}", test_name);
        self.memory.associate(
            &failure_id, &test_concept, 1.0
        ).await?;

        Ok(failure_id)
    }

    /// Find similar test failures
    pub async fn find_similar_failures(
        &self,
        error: &str,
    ) -> Result<Vec<TestFailure>, MemoryError> {
        let memories = self.memory.recall(error, 0.7, 5).await?;

        let mut failures = Vec::new();
        for entry in memories {
            let test_name = entry.context
                .split("test: ")
                .nth(1)
                .and_then(|s| s.split(", ").next())
                .unwrap_or("unknown")
                .to_string();

            failures.push(TestFailure {
                test_name,
                error: entry.context.clone(),
                solution: if entry.result != "unresolved" {
                    Some(entry.result.clone())
                } else {
                    None
                },
                similarity: entry.similarity,
            });
        }

        Ok(failures)
    }

    /// Check if a test is flaky (fails intermittently)
    pub async fn is_flaky_test(
        &self,
        test_name: &str,
    ) -> Result<bool, MemoryError> {
        let test_concept = format!("test::{}", test_name);

        // Get all failures for this test
        let failures = self.memory.framework()
            .get_associations(&test_concept).await?;

        if failures.len() < 3 {
            return Ok(false);  // Need multiple failures to detect flakiness
        }

        // Check if different errors (flakiness indicator)
        let mut error_types = std::collections::HashSet::new();
        for (failure_id, _) in failures {
            if let Some(concept) = self.memory.framework()
                .get_concept(&failure_id).await? {
                // Extract error type from context
                let error_type = extract_error_type(
                    &concept.context
                );
                error_types.insert(error_type);
            }
        }

        // Flaky if different error types
        Ok(error_types.len() > 1)
    }
}

pub struct TestFailure {
    pub test_name: String,
    pub error: String,
    pub solution: Option<String>,
    pub similarity: f32,
}

fn extract_error_type(context: &str) -> String {
    // Extract error type from context
    // e.g., "error: assertion failed" -> "assertion_failed"
    context.split("error: ")
        .nth(1)
        .and_then(|s| s.split(", ").next())
        .unwrap_or("unknown")
        .to_string()
}
```

---

## goap-planning

### Pattern: Action Effectiveness Tracking

**Use Case:** Track which GOAP actions lead to successful outcomes.

```rust
use skill_memory::SkillMemory;

pub struct GOAPMemory {
    memory: SkillMemory,
}

impl GOAPMemory {
    pub async fn initialize() -> Result<Self, MemoryError> {
        Ok(Self {
            memory: SkillMemory::initialize("goap-planning").await?,
        })
    }

    /// Remember action execution
    pub async fn remember_action(
        &self,
        action: &str,
        preconditions: &[(&str, bool)],
        effects: &[(&str, bool)],
        success: bool,
        duration_ms: u64,
    ) -> Result<String, MemoryError> {
        let context = format!(
            "action: {}, preconditions: {:?}",
            action, preconditions
        );

        let result = format!(
            "success: {}, effects: {:?}, duration_ms: {}",
            success, effects, duration_ms
        );

        self.memory.remember(
            "goap_action",
            &context,
            &result
        ).await
    }

    /// Find effective action sequences
    pub async fn find_effective_sequences(
        &self,
        goal: &str,
    ) -> Result<Vec<ActionSequence>, MemoryError> {
        // Query for actions leading to this goal
        let memories = self.memory.recall(
            &format!("goap action goal:{}", goal),
            0.6, 10
        ).await?;

        // Group into sequences by session/timestamp
        // ... implementation

        Ok(vec![])
    }
}

pub struct ActionSequence {
    pub actions: Vec<String>,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
}
```

---

## Common Patterns

### Pattern 1: Pre-Execution Context Query

Always query memory before executing to get context:

```rust
async fn execute_with_memory(
    memory: &SkillMemory,
    task: &Task,
) -> Result<TaskResult> {
    // 1. Query for relevant past operations
    let context = memory.recall(
        &task.description, 0.6, 5
    ).await?;

    // 2. Execute with context
    let result = execute_task(task, &context).await?;

    // 3. Remember this execution
    memory.remember(
        &task.operation_type,
        &task.description,
        &result.summary()
    ).await?;

    Ok(result)
}
```

### Pattern 2: Progressive Enhancement

Skills work without memory, enhanced when available:

```rust
async fn execute_skill(ctx: &SkillContext, task: &Task) {
    if let Some(memory) = ctx.memory() {
        // Enhanced execution with memory
        let context = memory.recall(...).await?;
        execute_with_context(task, context).await?;
    } else {
        // Fallback to basic execution
        execute_basic(task).await?;
    }
}
```

### Pattern 3: Association Chains

Build chains of related concepts:

```rust
// Error → Cause → Solution
memory.associate(error_id, cause_id, 0.95).await?;
memory.associate(cause_id, solution_id, 0.9).await?;

// Later: Follow chain
let causes = memory.related(error_id, 0.8).await?;
for (cause, _) in causes {
    let solutions = memory.related(&cause.id, 0.7).await?;
    // Process solutions
}
```

## Best Practices

1. **Namespace consistently**: Always use `skill::{skill_name}::` prefix
2. **Clean up temp data**: Remove intermediate concepts if not needed
3. **Set appropriate thresholds**: 0.6-0.8 for recall, 0.8+ for associations
4. **Limit top_k**: 5-10 results usually sufficient
5. **Remember selectively**: Not every operation, just key decisions
6. **Use metadata**: Store structured data alongside hypervectors
