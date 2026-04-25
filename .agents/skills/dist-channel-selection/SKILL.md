---
name: dist-channel-selection
description: Artifact-aware decision logic for selecting the correct distribution channel (cargo crate vs WASM npm vs CLI npm). Use when validating, installing, or publishing.
---

# Distribution Channel Selection

This repository publishes three distinct artifacts. They are NOT interchangeable. Before performing any install, publish, or verification task, you MUST identify which artifact is the target.

## Artifact Decision Matrix

| Artifact | Distribution Channel | Package/Crate Name | Primary Use Case |
|----------|----------------------|--------------------|------------------|
| **Rust Library** | crates.io / cargo | `chaotic_semantic_memory` | Native Rust development, performance-critical backends, crate validation. |
| **JS/WASM Library** | npm (WASM) | `@d-o-hub/chaotic_semantic_memory` | Web browsers, Node.js applications, TypeScript consumption. |
| **CLI Tool** | npm (CLI) | `@d-o-hub/csm` | End-user CLI installation, binary distribution via npm. |

---

## 1. Rust Crate Path (Source of Truth)

Use this path for repo-native developer workflows and core library validation.

### Canonical Commands
- **Check:** `cargo check`
- **Test:** `cargo test --all-features`
- **Build CLI (Local):** `cargo install --path . --features cli`
- **Publish Dry-run:** `cargo publish --dry-run`

### Decision Logic
Choose this path if:
- You are modifying `.rs` files in `src/`.
- You are adding a new feature to the core engine.
- You are fixing a bug in the HVec or Reservoir logic.

---

## 2. WASM npm Path

Use this path for browser/JS/TypeScript consumption and registry verification of the WASM bindings.

### Canonical Commands
- **Check Version:** `npm view @d-o-hub/chaotic_semantic_memory version`
- **Install Test:** `npm install @d-o-hub/chaotic_semantic_memory`
- **Build (Local):** `wasm-pack build --target web --out-dir wasm/pkg -- --features wasm`
- **Validation:** Verify `wasm/package.json` alignment.

### Decision Logic
Choose this path if:
- You are testing the JS API surface.
- You are investigating an issue with the WASM build.
- The task explicitly mentions "npm package for WASM" or "JS library".

---

## 3. CLI npm Path

Use this path for end-user CLI installation experience and binary distribution validation.

### Canonical Commands
- **Check Version:** `npm view @d-o-hub/csm version`
- **Install Test:** `npm install -g @d-o-hub/csm`
- **Check Binary:** `csm --version` / `csm --help`
- **Validation:** Verify `cli-npm/package.json` and postinstall behavior.

### Decision Logic
Choose this path if:
- You are testing how users install the `csm` tool globally via npm.
- You are validating the `postinstall.js` binary fetcher.
- The task explicitly mentions "CLI npm package" or binary distribution.

---

## Examples

### Rust Developer Flow
"I just added a new similarity metric to `src/hyperdim.rs`. How do I verify it?"
- **Artifact:** Rust Library
- **Channel:** cargo
- **Action:** `cargo test`

### JS/WASM Consumer Flow
"A web developer is reporting that `Similarity` enum is missing in the npm package."
- **Artifact:** JS/WASM Library
- **Channel:** `@d-o-hub/chaotic_semantic_memory`
- **Action:** `npm view @d-o-hub/chaotic_semantic_memory` and check `wasm/package.json`.

### CLI User Flow
"How do I install the latest CSM tool on my machine without compiling from source?"
- **Artifact:** CLI Tool
- **Channel:** `@d-o-hub/csm`
- **Action:** `npm install -g @d-o-hub/csm`

---

## Anti-Patterns (DO NOT DO)
- ❌ Using `@d-o-hub/csm` when the task is about the JS/WASM library.
- ❌ Recommending `npm install` for core Rust validation.
- ❌ Recommending `cargo install` for a JS consumer install path.
- ❌ Documenting "the npm package" as if there were only one.
