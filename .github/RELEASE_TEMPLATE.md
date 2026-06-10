# Release Notes Template

Use this template when writing curated release notes for a new fabio version.
The auto-generated changelog from `git-cliff` provides the raw commit list;
this template structures the **human-readable narrative** published on GitHub Releases.

---

## What's New

<!-- 1-3 sentence high-level summary of the release. What's the theme?
     What should users know at a glance? -->

This release brings <headline summary — e.g. "3 new item types, a major sync
upgrade with rsync-inspired flags, and enhanced error diagnostics">. <Optional
second sentence expanding on the most impactful change.>

### New Commands

<!-- List new command groups or subcommands added in this release.
     New item types and headline features ALWAYS go first. -->

**<Item Type / Command Group>**:
- `list`, `show`, `create`, `update`, `delete`, `get-definition`, `update-definition`
- Notable behavior or flags

### <Headline Feature 1>

Brief description of the most impactful feature. Include:
- Key capabilities added
- Example command(s) if applicable
- Any flags or options introduced

### <Headline Feature 2>

<!-- Repeat for each major feature worth calling out -->

### Improvements

- **<scope>**: Description of improvement ([`commit`](link))

### Bug Fixes

- **<scope>**: What was broken and how it's fixed

### Breaking Changes

<!-- Only include if there are breaking changes. Remove section otherwise. -->

- **<scope>**: What changed and migration path

### Documentation

- Description of documentation changes

### CI/CD

- Description of CI/CD improvements

### Stats

- N commits, N files changed, +N / -N lines
- Notable test coverage additions

**Full Changelog**: https://github.com/iemejia/fabio/compare/vPREVIOUS...vCURRENT

---

## Workflow

1. Generate the raw changelog: `git cliff --latest` (or `git cliff vPREV..vCURR`)
2. Review the grouped output for completeness (ensure no commits were missed)
3. Write the curated narrative using this template, grouping related changes
4. Lead with a descriptive paragraph summarizing the release theme
5. Put new item types and headline features FIRST
6. Publish via: `gh release edit vX.Y.Z --notes-file release-notes.md`
