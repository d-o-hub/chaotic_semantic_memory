//! Boundary tests for `secure_read_file` size check (line 159 `>` vs `>=`).
//!
//! `secure_read_file` is private; we exercise it through `import_json` which
//! calls it with `MAX_IMPORT_SIZE` (100 MB).  On Linux, `set_len` creates a
//! sparse file so the test is instant and uses negligible disk space.
//!
//! Killing the `>=` mutation:
//!   - exact size  → with `>` passes size check, fails at JSON parse (NOT a
//!     size error).  With `>=` it would fail with "exceeds maximum allowed size".
//!   - over size   → both operators reject with "exceeds maximum allowed size".

use chaotic_semantic_memory::prelude::*;
use std::io::Write;

/// 100 MB — must match `MAX_IMPORT_SIZE` in framework_validation.rs.
const LIMIT: u64 = 100 * 1024 * 1024;

#[tokio::test]
async fn exact_limit_size_not_rejected_as_oversized() {
    let tmp = std::env::temp_dir().join(format!("csm-boundary-exact-{}.bin", std::process::id()));
    // Sparse file: metadata reports LIMIT bytes but no real I/O.
    {
        let mut f = std::fs::File::create(&tmp).expect("create");
        f.set_len(LIMIT).expect("set_len");
        // Write a small valid-looking prefix so the file isn't all zeroes.
        f.write_all(b"{\"version\":\"t\",\"exported_at\":0,\"concepts\":[],\"associations\":[]}")
            .expect("write");
    }

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.import_json(tmp.to_str().unwrap(), false).await;
    let _ = std::fs::remove_file(&tmp);

    // With the correct `>` operator, a file exactly at the limit passes the
    // size check and fails later (JSON parse of the short prefix + zeroes).
    // If the mutant `>=` were applied, the error would mention "exceeds
    // maximum allowed size" — which must NOT happen.
    let err_msg = match result {
        Ok(_) => return, // even better — it succeeded entirely
        Err(e) => format!("{e}"),
    };
    assert!(
        !err_msg.contains("exceeds maximum allowed size"),
        "file exactly at the limit should NOT be rejected as oversized, got: {err_msg}"
    );
}

#[tokio::test]
async fn one_byte_over_limit_rejected() {
    let tmp = std::env::temp_dir().join(format!("csm-boundary-over-{}.bin", std::process::id()));
    {
        let f = std::fs::File::create(&tmp).expect("create");
        f.set_len(LIMIT + 1).expect("set_len");
    }

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.import_json(tmp.to_str().unwrap(), false).await;
    let _ = std::fs::remove_file(&tmp);

    let err_msg = match result {
        Ok(_) => panic!("one byte over limit must be rejected"),
        Err(e) => format!("{e}"),
    };
    assert!(
        err_msg.contains("exceeds maximum allowed size"),
        "one byte over limit should be rejected as oversized, got: {err_msg}"
    );
}
