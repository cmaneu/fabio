You are working on fabio, a Rust CLI for Microsoft Fabric.

The Microsoft Fabric REST API specs (github.com/microsoft/fabric-rest-api-specs) have been updated.

Here are the commits since our last sync:

${SPEC_COMMITS}

Here is a detailed summary of file changes:

${SPEC_CHANGES_SUMMARY}

The full diff is available at /tmp/spec-changes.diff and the full spec repo is at ./fabric-rest-api-specs/.

Your task:
1. Analyze the spec changes to identify new endpoints, modified request/response schemas, new API versions, or deprecated fields.
2. Compare against the current fabio implementation in src/commands/ and src/client.rs.
3. Make targeted improvements to fabio: add new subcommands, update existing request/response handling, fix field names, or add support for new parameters.
4. Update relevant tests in tests/ if you add or modify commands.
5. Before committing, run the mandatory pre-commit validation defined in AGENTS.md (section "Pre-Commit Validation (MANDATORY)"). All steps must pass with zero errors and zero warnings.
6. Write a file called /tmp/pr-body.md describing what was changed and why, referencing the spec commits.

Focus on high-impact changes: new endpoints that map to fabio command groups, breaking schema changes, and new required fields.

Follow all constraints and preferences defined in AGENTS.md, in particular the pre-commit validation rules and Windows-first compatibility requirements.
