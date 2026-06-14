# Security Policy

## Supported Versions

Only the latest release receives security fixes. Upgrade to the latest version before reporting.

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | No |

## Reporting a Vulnerability

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, report vulnerabilities privately using one of these methods:

1. **GitHub Security Advisories (preferred):** Go to [Security > Advisories > New draft advisory](https://github.com/iemejia/fabio/security/advisories/new) and submit a private report.

2. **Email:** Send details to the maintainer at the email listed in the commit history.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected version(s)
- Impact assessment (what an attacker could achieve)
- Suggested fix (if you have one)

### Response timeline

- **Acknowledgment:** Within 72 hours
- **Assessment:** Within 1 week
- **Fix (if confirmed):** Best effort, typically within 2 weeks for critical issues

### Scope

The following are in scope for security reports:

- Authentication token leakage or credential exposure
- Arbitrary code execution via crafted input
- Path traversal in file operations (upload, download, selfupdate)
- Binary replacement attacks (checksum bypass in `fabio upgrade`)
- Privilege escalation via profile/config manipulation
- Dependency vulnerabilities with a known exploit path

The following are out of scope:

- Denial of service via large inputs (expected CLI behavior)
- Issues requiring physical access to the machine
- Social engineering attacks
- Vulnerabilities in Microsoft Fabric APIs themselves (report to Microsoft)

## Security Design

- **No unsafe code:** `unsafe_code = "deny"` in Cargo.toml (exception: single-threaded `set_var` at startup)
- **Token storage:** Encrypted with DPAPI on Windows; file permissions 0600 on Unix
- **Binary updates:** SHA256 checksum verification before replacement
- **Dependencies:** Permissive licenses only; Dependabot + `cargo-audit` monitoring
- **TLS:** rustls (no OpenSSL on Linux/macOS for runtime)
