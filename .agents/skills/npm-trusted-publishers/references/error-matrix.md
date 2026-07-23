# npm Common Error Matrix

| Error Code | OIDC Status | Root Cause | Solution |
|------------|-------------|------------|----------|
| `E404 Not Found` | Signed ✓ | Workflow mismatch | Update npmjs Trusted Publisher workflow filename |
| `E404 Not Found` | Signed ✓ | Environment mismatch | Remove environment or match exactly |
| `E404 Not Found` | Signed ✓ | Package doesn't exist | Initial publish requires NPM_TOKEN |
| `EOTP Required` | N/A | Token-based publish | OIDC not attempted; use `--provenance` |
| `E403 Forbidden` | Not signed | Missing id-token permission | Add `id-token: write` to permissions |
| OIDC failed | Not signed | ACTIONS_ID_TOKEN_REQUEST_TOKEN unset | Check workflow permissions |
