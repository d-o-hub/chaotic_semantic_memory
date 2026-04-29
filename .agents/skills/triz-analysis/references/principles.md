# 40 TRIZ Principles for Software Engineering

Adapted from classical TRIZ for software development contexts.

## Core Principles (Most Common in Software)

### 1. Segmentation
Divide an object into independent parts.
- **Software**: Microservices, modules, functions, classes
- **Example**: Split monolith into bounded contexts

### 2. Taking out
Extract the disturbing part, keep only what's needed.
- **Software**: Extract method, separate concerns, dependency injection
- **Example**: Extract logging into separate concern

### 3. Local quality
Make each part perform differently in optimal conditions.
- **Software**: Specialized algorithms for different inputs, polymorphism
- **Example**: Different sorting algorithms based on data size

### 4. Asymmetry
Replace symmetry with asymmetry.
- **Software**: Different logic for edge cases, specialized handlers
- **Example**: Read-optimized vs write-optimized paths

### 5. Merging
Combine similar objects, perform parallel operations.
- **Software**: Batch operations, parallel processing, connection pooling
- **Example**: Batch database writes instead of individual inserts

### 6. Universality
Make a part perform multiple functions.
- **Software**: Multi-purpose interfaces, generic algorithms
- **Example**: A single cache that handles multiple data types

### 7. Nested doll
Place one object inside another.
- **Software**: Composition, wrapper patterns, middleware layers
- **Example**: Request handlers wrapped in logging, auth, metrics

### 8. Anti-weight
Counter heavy with light.
- **Software**: Offload to services, use indexes vs scans
- **Example**: Use bloom filter before expensive lookup

### 9. Preliminary anti-action
Pre-stress to handle stress.
- **Software**: Pre-computation, caching, warm-up routines
- **Example**: Pre-populate caches on startup

### 10. Preliminary action
Perform required action in advance.
- **Software**: Schema migrations, data migrations, pre-compilation
- **Example**: Compile templates at build time

### 11. Beforehand cushion
Prepare emergency measures beforehand.
- **Software**: Circuit breakers, fallbacks, retry policies
- **Example**: Default values for missing config

### 12. Inversion
Do the opposite of what's expected.
- **Software**: Inversion of control, reverse data flow, push vs pull
- **Example**: Dependency injection container inverts control

### 13. Spheroidality
Use curves instead of lines.
- **Software**: Circular buffers, round-robin, continuous integration
- **Example**: Ring buffer for streaming data

### 14. Curvature increase
Transition from linear to curved.
- **Software**: Non-linear algorithms, adaptive strategies
- **Example**: Exponential backoff

### 15. Dynamicity
Make objects movable/adaptable.
- **Software**: Configurable behavior, plugins, hot reloading
- **Example**: Feature flags for gradual rollout

### 16. Partial/excessive action
If 100% is hard, do more or less.
- **Software**: Approximation algorithms, sampling, thresholds
- **Example**: Probabilistic data structures (HyperLogLog)

### 17. Another dimension
Use multi-layer arrangements.
- **Software**: Layered architecture, multiple abstraction levels
- **Example**: Protocol stack (HTTP over TCP over IP)

### 18. Mechanical vibration
Use oscillation, frequency.
- **Software**: Polling, heartbeats, periodic jobs
- **Example**: Health check endpoints

### 19. Periodic action
Use periodic instead of continuous.
- **Software**: Batch processing, scheduled tasks, rate limiting
- **Example**: Background jobs instead of inline processing

### 20. Continuity of useful action
Carry on work without breaks.
- **Software**: Streaming, reactive systems, continuous deployment
- **Example**: Event stream processing

### 21. Rushing
Perform harmful operations fast.
- **Software**: Quick failure, fast timeouts, circuit open
- **Example**: Fail-fast validation

### 22. Convert harm into benefit
Use negative effects positively.
- **Software**: Chaos engineering, fuzzing, error as data
- **Example**: Use test failures to improve coverage

### 23. Feedback
Introduce feedback loops.
- **Software**: Monitoring, metrics, adaptive algorithms
- **Example**: Auto-scaling based on CPU metrics

### 24. Intermediary
Use an intermediate carrier.
- **Software**: Message queues, event buses, adapters
- **Example**: Kafka between producers and consumers

### 25. Self-service
Make the object serve itself.
- **Software**: Self-healing systems, auto-configuration, reflection
- **Example**: Auto-tuning database parameters

### 26. Copying
Use copies instead of originals.
- **Software**: Caching, replicas, snapshots, prototypes
- **Example**: Read replicas for database scaling

### 27. Cheap disposables
Replace expensive with cheap copies.
- **Software**: Short-lived connections, ephemeral containers, serverless
- **Example**: Lambda functions

### 28. Mechanics substitution
Replace mechanical with sensory.
- **Software**: Observability, tracing, logging
- **Example**: Distributed tracing instead of step debugging

### 29. Pneumatic/hydraulic construction
Use gas/liquid for solid.
- **Software**: Fluid interfaces, reactive streams, flow-based programming
- **Example**: RxJS observables

### 30. Flexible shells/thin films
Use flexible containers.
- **Software**: Containers, sandboxes, isolation boundaries
- **Example**: Docker containers

### 31. Porous materials
Add holes or cavities.
- **Software**: Sparse data structures, lazy loading, virtual scrolling
- **Example**: Sparse matrix representation

### 32. Color changes
Change color or transparency.
- **Software**: Visibility toggles, feature flags, A/B testing
- **Example**: Dark mode, highlight errors

### 33. Homogeneity
Make interacting objects from same material.
- **Software**: Consistent APIs, unified interfaces, polymorphism
- **Example**: Repository pattern with consistent interface

### 34. Discarding and recovering
Remove what's done, restore what's needed.
- **Software**: Garbage collection, connection pooling, object reuse
- **Example**: Object pool pattern

### 35. Parameter changes
Change physical state or concentration.
- **Software**: Serialization formats, compression, encoding
- **Example**: JSON vs Protocol Buffers

### 36. Phase transitions
Use phenomena during transitions.
- **Software**: State machines, transaction boundaries, commits
- **Example**: Two-phase commit

### 37. Thermal expansion
Use expansion/contraction.
- **Software**: Elastic scaling, auto-scaling, shrinking
- **Example**: Kubernetes HPA

### 38. Strong oxidants
Accelerate processes.
- **Software**: Parallelization, SIMD, GPU compute
- **Example**: Rayon for parallel iterators

### 39. Inert atmosphere
Create neutral environment.
- **Software**: Sandboxing, isolation, namespace separation
- **Example**: Process isolation in browsers

### 40. Composite materials
Use composites instead of uniform.
- **Software**: Hybrid algorithms, mixed strategies, ensemble methods
- **Example**: Hybrid BM25 + semantic search

---

## Quick Reference for Common Contradictions

| Contradiction | Suggested Principles |
|---------------|---------------------|
| Speed vs Simplicity | #1, #2, #16, #26 |
| Features vs Stability | #15, #11, #35, #40 |
| Performance vs Memory | #8, #16, #26, #31 |
| Flexibility vs Complexity | #1, #2, #12, #33 |
| Security vs Usability | #3, #9, #11, #24 |
| Reliability vs Speed | #11, #15, #23, #34 |