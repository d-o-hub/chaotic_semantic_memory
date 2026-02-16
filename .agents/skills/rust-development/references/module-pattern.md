# Module Pattern

Use this as the default structure.

```rust
//! Module documentation

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Component {
    // fields
}

impl Component {
    pub async fn new() -> Result<Self, Error> {
        // implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic() {
        // test
    }
}
```

Conventions:
- Return `Result<T, Error>` from public APIs.
- Use Tokio for async I/O and Rayon for CPU parallelism.
- Guard thread-dependent features in WASM with cfg gates.
