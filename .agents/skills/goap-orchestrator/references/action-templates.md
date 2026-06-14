# Action Templates

## Extract Crate Action

```yaml
- name: extract_<crate_name>
  preconditions:
    source_modules_exist: true
    dependency_crate_ready: true
  effects:
    <crate_name>_extracted: true
    source_modules_moved: true
  cost: 8
  steps:
    1. Create crates/<crate_name>/Cargo.toml
    2. Move source modules to crates/<crate_name>/src/
    3. Update workspace members in root Cargo.toml
    4. Update import paths in main crate
    5. Add integration tests
    6. Verify cargo build -p <crate_name>
    7. Verify cargo test -p <crate_name>
```

## CI Update Action

```yaml
- name: update_ci_for_workspace
  preconditions:
    workspace_crates_ready: true
  effects:
    ci_workspace_aware: true
    per_crate_tests: true
  cost: 4
  steps:
    1. Update .github/workflows/ci.yml
    2. Add per-crate test jobs
    3. Add WASM32 compilation checks
    4. Update pre-release-gate.yml
    5. Verify CI passes
```

## Documentation Regeneration Action

```yaml
- name: regenerate_docs
  preconditions:
    workspace_finalized: true
  effects:
    docs_current: true
    llms_txt_updated: true
  cost: 2
  steps:
    1. Run gen-llms-txt.sh
    2. Update export.json
    3. Verify docs match workspace structure
```

## Branch & PR Action

```yaml
- name: create_feature_branch
  preconditions:
    main_branch_clean: true
  effects:
    feature_branch_created: true
    changes_staged: true
  cost: 1
  steps:
    1. git checkout -b feat/<scope>-<description>
    2. Implement changes
    3. git add <files>
    4. git commit -m "<type>(<scope>): <summary>"
    5. git push origin <branch>
    6. gh pr create --title "..." --body "..."
    7. gh pr checks --watch
```
