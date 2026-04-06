# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

**GitHub Security Advisories**
- Go to the repository's **Security** tab
- Click **Report a vulnerability**
- Follow the guided process

### Response Timeline

| Phase | Timeline |
|-------|----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 7 days |
| Patch release | Within 30 days (critical), 90 days (standard) |
| Public disclosure | After patch release |

### Security Best Practices

When using this crate:

1. **Keep dependencies updated** - Run `cargo audit` regularly
2. **Enable all security features** in production
3. **Validate all inputs** before processing
4. **Use TLS** for all network communications
5. **Review the code** if using in security-critical contexts

## Known Security Considerations

- WASM target uses `getrandom` with JavaScript fallback
- Database connections should use TLS in production
- Hypervector operations are deterministic (no cryptographic guarantees)

## Acknowledgments

We thank all security researchers who responsibly disclose vulnerabilities.
