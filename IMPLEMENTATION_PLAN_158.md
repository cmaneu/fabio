# Implementation plan: tenant admin settings export, plan, and apply

Issue: [iemejia/fabio#158](https://github.com/iemejia/fabio/issues/158)

## Goal and scope

Add a reviewable YAML workflow for tenant settings: export the current configuration,
compare an edited export with the live tenant, and apply only the planned changes.
Every networked phase must support checkpointing and `--continue` so the reported
25-requests-per-hour limit does not require starting over.

This document is the deliverable of the planning task; it does not implement commands
or change a live tenant. Keep it during implementation, then delete it in the final
cleanup step below. Existing individual settings commands remain compatible.
Capacity, domain, and workspace delegated overrides are outside this issue's scope.

## Existing implementation and constraints

- `/home/runner/work/fabio/fabio/src/commands/admin/mod.rs` defines `AdminCommand`
  and dispatches to the existing directory module.
- `/home/runner/work/fabio/fabio/src/commands/admin/tenant_settings.rs` lists
  `/admin/tenantsettings` using the `tenantSettings` response array and updates one
  setting through `POST /admin/tenantsettings/{settingName}/update`.
- `/home/runner/work/fabio/fabio/.agents/API-BEHAVIORS-DISCOVERED.md` documents that
  an update returns all settings in the same group, not just the requested setting.
  Match the response by `settingName`; do not assume its first element is the result.
- `/home/runner/work/fabio/fabio/src/client.rs` owns authentication, HTTPS validation,
  readonly enforcement, pagination, and HTTP errors. Its ordinary POST helper retries
  throttled requests up to three times, with waits capped at 300 seconds. Its
  all-pages list helper does not expose accumulated pages when a later page fails.
  These behaviors need an opt-in resumable path, not a global behavior change.
- `/home/runner/work/fabio/fabio/Cargo.toml` already includes `serde`, `serde_yaml`,
  `sha2`, `chrono`, and `fs2`, plus `wiremock` and `tempfile` for tests. Reuse existing
  dependencies and conventions rather than introducing a separate client or tool.
- `/home/runner/work/fabio/fabio/tests/e2e_admin.rs` already covers tenant settings
  listing, update dry-runs, and an ignored live update round-trip.

## 1. Confirm the API contract

- [ ] Check the current official Fabric list/update tenant-settings specifications
  before implementing. Confirm supported writable fields, required fields, nullable
  values, security-group restrictions, delegation fields, pagination, and whether
  omitted fields are preserved or reset.
- [ ] Verify the issue's quota and its scope: reads versus updates, caller versus
  tenant, and how `Retry-After` is returned. Treat 25 requests/hour as the design
  constraint, not proof that the CLI knows the remaining shared quota.
- [ ] Confirm how to obtain the effective authenticated tenant identity without an
  extra admin request. Bind plans and checkpoints to that identity and the validated
  API endpoint; do not rely only on a profile name.
- [ ] Determine whether the API offers conditional updates or idempotency support.
  Document limitations if it does not; do not promise transactional or exactly-once
  execution across ambiguous network failures.
- [ ] Record newly verified behaviors in
  `/home/runner/work/fabio/fabio/.agents/API-BEHAVIORS-DISCOVERED.md`.

## 2. Define the command and file contracts

Use flat admin subcommands consistent with the existing settings commands:

| Command | Fresh operation | Result |
| --- | --- | --- |
| `admin export-tenant-settings` | Required `--out` YAML destination | Complete tenant settings export |
| `admin plan-tenant-settings` | Required `--file` desired-state YAML and `--out` plan destination | YAML differences and proposed updates |
| `admin apply-tenant-settings` | Required `--file` reviewed plan YAML | Structured execution summary |

- [ ] All three commands accept `--continue <checkpoint-file>` as an alternative
  to fresh-operation inputs. Allow `--continuation-file` to select a checkpoint path
  for a fresh run; otherwise choose and report a deterministic sibling path beside
  the output artifact or input plan. Reject conflicting fresh/resume arguments.
- [ ] Keep tenant-scoped commands independent of `--workspace`. Do not shadow
  global flags, particularly `--output`, `--query`, `--force`, or
  `--continuation-token`; the latter is a service pagination cursor, not a workflow
  checkpoint.
- [ ] Write YAML artifacts to files, while stdout continues to use normal structured
  output helpers. Summaries include artifact paths, operation status, relevant
  counts, warnings, and a `destructive` boolean for plans and batch execution.
  Diagnostics and machine-readable failures remain on stderr.
- [ ] Fetch complete snapshots regardless of the default list-page behavior.
  Presentation flags such as `--limit` and `--query` must not truncate or transform
  saved configuration or execution inputs. Reject a user-supplied pagination start
  token for fresh exports/plans rather than silently producing a partial snapshot.
- [ ] Define versioned YAML schemas for desired settings, plans, and checkpoints.
  Sort settings deterministically by exact `settingName` for readable diffs.
  Separate display/capability metadata from the explicitly supported writable values.
- [ ] Exports include all returned settings, including descriptive metadata for
  non-writable settings. Reject requested edits to unsupported fields/settings with
  actionable errors; never forward the entire list response as an update body.
- [ ] A plan records tenant identity, schema version, creation time, source digest,
  baseline values, desired values, field-level differences, and ordered updates.
  Keep unchanged counts in the summary; include no update operation for a no-op.
- [ ] Validate schemas, duplicate YAML keys and setting names, unknown fields,
  value types, and supported versions before network calls where possible.
  In desired state, omitted settings/optional fields mean unmanaged or unchanged;
  explicit empty collections mean clear only where the API permits it.
  Missing settings must never imply deletion or disabling.
- [ ] Normalize only semantically unordered collections, such as security-group
  memberships by stable IDs, to avoid false differences. Do not conflate missing,
  null, false, and empty values when the API distinguishes them.
- [ ] Refuse output/input/checkpoint path collisions and unrelated existing output
  files. Any supported overwrite must use the existing explicit safety conventions.
  Use cross-platform path handling and failure-safe file replacement.

## 3. Add checkpoint-aware HTTP and persistence support

- [ ] Add the smallest opt-in request policy to `FabricClient` needed to surface
  throttling immediately for this workflow, instead of sleeping/retrying through an
  hourly quota. Preserve existing behavior for other callers and retain auth refresh,
  trusted-URL validation, readonly checks, and admin-aware error enrichment.
- [ ] Preserve rate-limit response metadata through error enrichment and rendering.
  Support valid `Retry-After` delay/date forms, with safe handling of missing,
  malformed, or excessive values. Report the next eligible retry time when known.
- [ ] Expose one-page list results with both continuation token and continuation URI,
  reusing existing pagination logic. Checkpoint each successful page, so throttling
  on a later page does not lose the completed pages. Validate resumed URLs before
  sending credentials, including an appropriate endpoint/origin restriction.
- [ ] Use one shared workflow checkpoint model for all three commands, with operation
  kind, phase, tenant/endpoint binding, input digest, resolved artifact paths,
  accumulated snapshot pages, next-page cursor, exact pending updates, acknowledged
  successes, in-flight/uncertain work, and retry timing.
- [ ] Persist an initial checkpoint before the first request, and update it after
  each successful page or setting update. Record an update as in flight before
  sending it. Stop immediately if durable progress cannot be saved.
- [ ] Use a cross-platform, single-writer lock and atomic replacement strategy.
  A process interruption must leave a readable prior checkpoint; a second process
  must not apply the same pending work concurrently.
- [ ] Checkpoints must not contain authentication tokens, credentials, or authorization headers.
  Treat tenant/group identifiers as sensitive configuration, avoid unnecessary
  logging, and validate local state as untrusted input. Digests detect accidental
  mismatch; they are not proof against deliberate edits.
- [ ] On throttling, save progress and return the existing `RATE_LIMITED` error
  classification and exit code, with a checkpoint path, completion counts, and an
  exact resume hint. Never report partial execution as success.
- [ ] `--continue` validates version, operation, tenant, endpoint, file integrity,
  and input/plan digest before resuming. Reject changed plans or incompatible state.
  Resume at the recorded cursor/pending update, not at the beginning.
- [ ] Do not repeat acknowledged successful requests. If a connection loss or crash
  leaves an update's result uncertain, reconcile that setting against current state
  before deciding whether it needs a write; never blindly replay it. This read is
  recovery work, not a guarantee of exactly-once delivery.
- [ ] Preserve progress on non-throttling failures too, but surface the actual error
  rather than labeling it rate limiting. Finalize artifacts only after all pages or
  updates finish; retain a terminal completion record so reusing a completed
  checkpoint cannot reapply changes.

## 4. Implement export, planning, and application

### Export

- [ ] Read the full tenant snapshot with one paginated listing, not one request per
  setting. Save each completed page through the shared checkpoint mechanism.
- [ ] Serialize a deterministic, schema-valid export that can be edited and used
  directly as planning input. Keep partial data in checkpoint state, not in a
  file that appears to be a complete export.

### Plan

- [ ] Validate and snapshot the desired-state input, then fetch current settings
  using the same resumable reader.
- [ ] Match by `settingName`, validate requested settings against live capabilities,
  and compare writable values only. Reject unknown settings, unsupported edits,
  invalid group scopes, and ambiguous intent before producing an executable plan.
- [ ] Produce human-reviewable before/after field differences, exact proposed
  payloads, deterministic execution order, and change/no-op counts. A fresh export
  against unchanged tenant state produces zero updates.
- [ ] Mark governance changes with a conservative destructive warning and explicit
  tenant scope. Planning never issues update requests.

### Apply

- [ ] Validate the entire plan and checkpoint destination before any mutation.
  Accept only explicit, supported setting updates; reject arbitrary HTTP methods,
  URLs, wildcards, duplicate operations, and payloads inconsistent with the plan.
  An empty valid plan completes as a no-op.
- [ ] Provide an offline dry-run preview after local validation, using the exact
  `admin apply-tenant-settings` operation key. Include affected settings, counts,
  tenant scope, and the destructive confirmation signal; do not create execution
  checkpoints or send mutation requests during dry-run.
- [ ] For real execution, perform a resumable bulk preflight read to detect drift:
  already-desired values need no write, baseline matches may proceed, and conflicting
  values require replanning. On resume, preserve acknowledged work while refreshing
  current state for pending entries when necessary, without changing the approved
  baseline values. Document the remaining race if conditional updates are unavailable.
- [ ] Apply pending updates sequentially through `FabricClient`; do not use the
  parallel retry framework for this low-quota API. Use conditional updates if
  supported. Avoid automatic replay of ambiguous failed writes.
- [ ] Confirm each acknowledged response for the requested setting, update progress,
  and stop on the first failure. Do not count group siblings as completed operations.
- [ ] Do not silently roll back partial execution: rollback also consumes quota and
  could overwrite later edits. Preserve before values for a separately reviewed
  restoration plan and report completed, skipped, pending, and uncertain work.
- [ ] Enforce global `--readonly` through the client and mark apply as mutating and
  destructive in generated command metadata. Do not introduce prompts or approval
  bypass flags; any necessary bypass must follow existing agent-notice conventions.

## 5. Keep the implementation focused

- [ ] Extend the admin enum/dispatch in
  `/home/runner/work/fabio/fabio/src/commands/admin/mod.rs`.
  Keep workflow handlers under that directory, grouped by export/plan/apply with
  shared typed data/checkpoint helpers. Leave existing settings and override handlers
  intact, and keep each command source file below the repository's size limit.
- [ ] Limit shared HTTP/error changes to the capabilities identified above in
  `/home/runner/work/fabio/fabio/src/client.rs` and, if required,
  `/home/runner/work/fabio/fabio/src/errors.rs` and
  `/home/runner/work/fabio/fabio/src/output.rs`. Preserve existing envelopes and exit
  codes. No new top-level command group or unrelated refactoring is needed.

## 6. Test the acceptance criteria and failure paths

| Acceptance criterion | Required evidence |
| --- | --- |
| Export all settings to reusable YAML | Multi-page export, YAML round-trip, editable desired input, unchanged re-plan |
| Plan shows differences and proposed updates | Scalar/group changes, readable before/after values, stable ordering, no-op detection |
| Apply changes the tenant | Exact mocked POST paths/bodies, structured results, guarded live round-trip |
| Throttled operations resume without replaying successes | Export, plan, and apply each pause on a later request and resume with exact request-count assertions |

- [ ] Add inline unit tests for serialization, normalization, diffing, strict
  validation, schema versions, tenant/digest mismatches, readonly metadata, and
  retry-time parsing.
- [ ] Extend `/home/runner/work/fabio/fabio/tests/e2e_admin.rs` with non-ignored,
  hermetic `wiremock` CLI tests. Cover 429/430, first-request throttling, repeated
  throttling across resumes, token- and URI-based pagination, malformed responses,
  grouped update responses, stale plans, and unchanged settings making zero writes.
- [ ] Test offline dry-run, readonly rejection, structured errors and exit codes,
  quiet/output formatting, presentation flags not altering YAML, wrong-tenant
  rejection, unsafe resumed URLs, and invalid input failing before mutation.
- [ ] Exercise crashes/uncertain acknowledgments, checkpoint-write failures,
  unwritable paths, conflicting paths, corrupted files, concurrent resumes, and
  completed-checkpoint reuse. Verify acknowledged updates are never replayed.
- [ ] Keep existing client retry behavior and individual admin commands covered by
  regression tests. Extend destructive guardrail coverage for the new apply command.
- [ ] Add ignored, explicitly opted-in live tests in a designated test tenant.
  Export and verify a no-op plan first; then change only an administrator-approved
  setting, verify it, and restore its actual captured original values even on failure.
  Do not assume any setting starts disabled or deliberately exhaust a live quota.
  Reproduce throttling through mocks.
- [ ] Before considering implementation complete, run formatting, warning-free
  Clippy, the full test suite, schema/skill drift gates, the cross-target check, and
  documentation checks. Run the approved live validation separately; if credentials,
  approval, or quota are unavailable, report it as blocked rather than claiming a pass.

## 7. Update documentation and agent discovery

- [ ] Regenerate command metadata and sub-skills using the existing generators.
  Verify mutability, destructiveness, flag discovery, and global-flag collision tests.
  Do not hand-edit generated command tables or generated reference pages.
- [ ] Add authored admin guidance, a workflow recipe, and registered output examples
  under `/home/runner/work/fabio/fabio/src/commands/context/data/`. Explain YAML
  editing, plan review, quota handling, resume versus pagination, partial failures,
  state-file sensitivity, and recovery limitations.
- [ ] Update `/home/runner/work/fabio/fabio/README.md` and the appropriate authored
  guides under `/home/runner/work/fabio/fabio/docs/src/content/docs/`.
  Add promptfoo cases for command selection, required file flags, resume behavior,
  and tenant-wide safety warnings.
- [ ] Record decisions and new files in
  `/home/runner/work/fabio/fabio/.agents/KEY-DECISIONS.md` and
  `/home/runner/work/fabio/fabio/.agents/RELEVANT-FILES.md`, without duplicating
  runtime API details or expanding the root agent instructions.

## 8. Final acceptance and cleanup

- [ ] Verify every issue acceptance criterion against the automated and approved
  live evidence, and resolve review/security findings.
- [ ] Confirm no credentials, real tenant exports, plans, or continuation artifacts
  are included in commits; scan changed files for secrets.
- [ ] Move any enduring design decisions into the appropriate existing documentation.
- [ ] **Delete `/home/runner/work/fabio/fabio/IMPLEMENTATION_PLAN_158.md` as the final
  implementation cleanup step**, once code, tests, and permanent documentation are
  complete. Do not delete it during this planning-only task.
