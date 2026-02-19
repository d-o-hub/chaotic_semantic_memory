# [ADR-0026] Namespace Isolation (Soft Multi-Tenancy)

## Status
Deferred (Post-1.0)

**Rationale**: Analysis Swarm Consensus (2026-02-17) determined multi-tenancy is not required for 1.0. Current system supports single-tenant use cases fully. Multi-tenant deployments can use separate database instances as workaround.

## Context and Problem Statement

Production deployments often need to serve multiple users, agents, or contexts from a single instance:
- SaaS platform with multiple customer tenants
- Multi-agent system with isolated agent memories
- User-specific recommendations without cross-contamination
- Testing vs production data separation

Current implementation has no isolation - all concepts share a single namespace, creating risks:
- ID collisions between tenants (user_123 exists in both tenant A and B)
- Data leakage through similarity search
- No way to scope operations to a subset of concepts

## Decision Drivers

1. **ID Isolation**: Same ID can exist in different namespaces
2. **Query Isolation**: Search only returns concepts from same namespace
3. **Association Isolation**: Cross-namespace associations disallowed
4. **Administrative Visibility**: Root/admin can see all namespaces
5. **Zero Overhead**: Single-tenant use case pays no penalty
6. **Migration Path**: Existing single-tenant data migrates seamlessly

## Considered Options

### Option 1: Separate Database per Tenant
Each tenant gets own SQLite file or Turso database.

**Pros:** Complete isolation, simple security model  
**Cons:** Connection overhead, no shared infrastructure, complex routing

### Option 2: Table Prefix/Namespace Column (Chosen)
Namespace as column/filter in shared tables.

**Pros:** Single connection pool, efficient resource use, cross-namespace ops possible  
**Cons:** Application-level enforcement (not database-level), query complexity

### Option 3: Row-Level Security (RLS)
Use SQLite RLS or Turso RLS features.

**Pros:** Database-level enforcement  
**Cons:** libsql support unclear, complex policy management

## Decision Outcome

Chosen: **Option 2 - Namespace Column with Application-Level Filtering**

### Design

#### 1. Data Model Changes

```rust
// In Concept struct
pub struct Concept {
    pub id: String,                    // User-provided ID (unique within namespace)
    pub namespace: String,             // NEW: "default" for backward compatibility
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,
}

// Internal storage uses fully-qualified ID
fn fully_qualified_id(namespace: &str, id: &str) -> String {
    format!("{}::{}", namespace, id)
}
```

#### 2. Database Schema

```sql
-- Add namespace column to concepts
ALTER TABLE concepts ADD COLUMN namespace TEXT DEFAULT 'default';
CREATE INDEX idx_concepts_namespace ON concepts(namespace);
CREATE UNIQUE INDEX idx_concepts_namespace_id ON concepts(namespace, id);

-- Add namespace to associations (implicit via concept lookup)
-- Associations remain scoped by from_id lookup
```

#### 3. API Design

```rust
// New builder method
impl FrameworkBuilder {
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }
}

// All existing APIs automatically use configured namespace
impl ChaoticSemanticFramework {
    // inject_concept, probe, associate, etc. 
    // all scoped to self.config.namespace
    
    /// Cross-namespace query (admin only)
    pub async fn probe_all_namespaces(
        &self,
        query: HVec10240,
        top_k: usize,
    ) -> Result<Vec<(String, String, f32)>> {
        // Returns (namespace, id, score) tuples
    }
    
    /// List all namespaces (admin only)
    pub async fn list_namespaces(&self) -> Result<Vec<String>>;
    
    /// Migrate concept between namespaces
    pub async fn move_to_namespace(
        &self,
        id: &str,
        target_namespace: &str,
    ) -> Result<()>;
}
```

#### 4. Namespace-Aware Operations

```rust
impl Singularity {
    fn inject(&mut self, 
        concept: Concept,
        current_namespace: &str
    ) -> Result<()> {
        // Store with fully-qualified ID internally
        let fqid = fully_qualified_id(current_namespace, &concept.id);
        self.concepts.insert(fqid, concept);
    }
    
    fn find_similar(
        &self, 
        query: &HVec10240, 
        top_k: usize,
        namespace: &str,
    ) -> Vec<(String, f32)> {
        // Only search concepts in this namespace
        self.concepts.iter()
            .filter(|(fqid, _)| fqid.starts_with(&format!("{}::", namespace)))
            .map(|(_, c)| (c.id.clone(), query.cosine_similarity(&c.vector)))
            .collect()
    }
}
```

#### 5. WASM Compatibility

```rust
#[wasm_bindgen]
impl WasmFramework {
    /// Create framework with namespace
    pub async fn new_with_namespace(namespace: String) -> Result<WasmFramework, JsValue>;
    
    /// All other methods automatically use the namespace
}
```

### Positive Consequences

1. **True Multi-Tenancy**: Complete data isolation between namespaces
2. **Resource Efficiency**: Single connection pool, shared cache
3. **Flexible Deployment**: One instance serves many customers
4. **Admin Visibility**: Cross-namespace operations for management
5. **Migration Path**: Existing data uses "default" namespace automatically
6. **No Breaking Changes**: Single-tenant code works unchanged

### Negative Consequences

1. **Application Enforcement**: Relies on correct namespace filtering in code
2. **Storage Overhead**: +namespace string per concept (deduplicated via interning possible)
3. **Query Complexity**: Every query needs namespace filter
4. **No Cross-Namespace Associations**: By design, but limits some use cases

### Mitigations

1. **Enforcement:** Type-safe namespace passing; compile-time guarantees
2. **Storage:** Use small-string optimization; most namespaces are short
3. **Queries:** Index on (namespace, id); filter is O(1) lookup
4. **Cross-Namespace:** Can be added later via explicit "shared" namespace

## Implementation Phases

### Phase 1: Core Namespace Support
1. Add namespace field to Concept
2. Update storage to use fully-qualified IDs
3. Add namespace to FrameworkConfig
4. Filter all queries by namespace

### Phase 2: Persistence
1. Database migration for namespace column
2. Update save/load to include namespace
3. Composite index on (namespace, id)

### Phase 3: Admin APIs
1. Cross-namespace query
2. List namespaces
3. Move between namespaces

### Phase 4: WASM
1. Expose namespace constructor
2. Ensure namespace isolation in browser

## Security Considerations

**Not a Security Boundary:** This is soft isolation for data organization, not a security mechanism. For true tenant isolation, use:
- Separate database instances
- Row-level security policies
- Application-level authorization

**Recommended Pattern:**
```rust
// Middleware validates user can access namespace
fn authorize_namespace(user: &User, namespace: &str) -> Result<()> {
    if user.allowed_namespaces.contains(namespace) {
        Ok(())
    } else {
        Err(Unauthorized)
    }
}
```

## LOC Budget

- singularity.rs: +30 lines (FQID handling, filtering)
- framework.rs: +25 lines (config, namespace parameter)
- persistence.rs: +20 lines (schema, queries)
- wasm.rs: +10 lines (constructor)

**Total: ~85 lines** - Within constraints

## Links

- Related ADRs:
  - ADR-0024: Concept Expiration (complementary for tenant TTL)
  - ADR-0005: Persistence Connection Model (connection pooling)
