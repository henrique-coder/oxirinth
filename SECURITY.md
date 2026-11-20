# Security Policy

## Supported Versions

Security fixes are applied to the latest released version of `oxirinth`. Older versions may not receive patches.

## Reporting a Vulnerability

Do not disclose security vulnerabilities publicly.

Preferred reporting path:

1. Use GitHub private vulnerability reporting if it is enabled for this repository.
2. If private reporting is not available, open a minimal public issue asking for private maintainer contact. Do not include exploit details, Modrinth Personal Access Tokens (`api_key`), request headers with credentials, local cache files, or reproduction payloads in the public issue.

Please include privately:

- A clear description of the vulnerability.
- A minimal reproduction when possible.
- Affected versions or commits.
- Potential impact.
- Any suggested fix or mitigation.

## Sensitive Data

Never include real Modrinth Personal Access Tokens, `Authorization` headers, private project drafts, or raw `http_cache.redb` database files in public issues, discussions, pull requests, or logs.

## Scope

In scope:

- Leaks of `api_key` values or authentication headers in logs, exceptions, or disk caches.
- Cache isolation flaws between authenticated and anonymous sessions.
- Uncaught Rust panics across the PyO3 FFI boundary or memory/safety bugs in `oxirinth._core`.
- Packaging or CI issues that could compromise published wheels or source distributions.

Out of scope:

- Vulnerabilities in Modrinth itself.
- Abuse of Modrinth API rate limits or account permissions.
- Social engineering, spam, or denial-of-service testing.
