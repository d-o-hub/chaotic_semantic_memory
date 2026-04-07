## 2025-05-15 - [Concept ID Control Character Injection]
**Vulnerability:** Concept IDs could contain control characters (null bytes, newlines, etc.), potentially leading to injection vulnerabilities in downstream systems (logs, terminal output, file systems).
**Learning:** Initial validation was fragmented between CLI and framework, focusing only on length and emptiness.
**Prevention:** Centralized validation in `src/framework_validation.rs` using `char::is_control()` to reject dangerous characters across all input vectors (CLI, API, imports).
