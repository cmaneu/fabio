You are working on fabio, a Rust CLI for Microsoft Fabric.

The Microsoft Fabric REST API specs (github.com/microsoft/fabric-rest-api-specs) have been updated.
Here is a summary of recent changes:

${SPEC_CHANGES_SUMMARY}

The full diff is available at /tmp/spec-changes.diff and the full spec repo is at ./fabric-rest-api-specs/.

Your task:
1. Analyze the spec changes to identify new endpoints, modified request/response schemas, new API versions, or deprecated fields.
2. Compare against the current fabio implementation in src/commands/ and src/client.rs.
3. Make targeted improvements to fabio: add new subcommands, update existing request/response handling, fix field names, or add support for new parameters.
4. Update relevant tests in tests/ if you add or modify commands.
5. Ensure the code compiles (run 'cargo check') and passes clippy ('cargo clippy').
6. Write a file called /tmp/pr-body.md describing what was changed and why, referencing the spec commits.

Focus on high-impact changes: new endpoints that map to fabio command groups, breaking schema changes, and new required fields. Skip cosmetic or documentation-only spec changes.

Important constraints:
- Rust edition 2024, rust-version 1.85
- clippy pedantic+nursery with zero warnings
- All output uses the JSON envelope pattern (see src/output.rs)
- Follow existing code patterns in src/commands/ for new subcommands
- Use Path::new().join() for filesystem paths (Windows compatibility)
