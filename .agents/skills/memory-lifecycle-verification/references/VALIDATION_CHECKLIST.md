# Memory Lifecycle Checklist

## Save

- [ ] concept IDs were injected successfully
- [ ] associations were created successfully
- [ ] concepts are discoverable through probe

## Load

- [ ] export file exists and is non-empty
- [ ] import succeeds into a clean database
- [ ] loaded records preserve IDs and metadata

## Archive

- [ ] archive marker or archive command output is recorded
- [ ] archived target ID is referenced explicitly
- [ ] archive timestamp exists

## Delete

- [ ] deleted/tombstoned IDs are not returned as active
- [ ] associations to deleted IDs are removed or blocked
- [ ] no file/DB orphan mismatch remains

## Cross-check

- [ ] file artifacts and DB row counts are consistent
- [ ] rerunning verification is idempotent
- [ ] final evidence bundle is attached to PR/release notes
