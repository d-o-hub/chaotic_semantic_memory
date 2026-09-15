//! Persistence layer using libSQL (SQLite/Turso). Auto-migrations, version retention, FK enabled.
//!
//! Canonical implementation lives in `csm-persistence` (ADR-0094); this module
//! keeps the root public path stable and re-exports the owner's type. The root
//! retains no second implementation: concept/association CRUD, the migration
//! ladder, version history, index snapshots, and the canonical graph all run
//! from the crate.

pub use csm_memory::ConceptVersion;
pub use csm_persistence::Persistence;
