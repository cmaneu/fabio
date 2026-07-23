---
title: CLI reference
description: Search every Fabio command group, subcommand, flag, and example.
---

This reference is generated from the same command schema Fabio exposes through `fabio context agent`. It stays aligned with the Clap command definitions and includes every command group's help content.

Use the search field in the header to find a command, resource, or flag across the entire site. Browse **Commands** in the sidebar to inspect a group.

## Command shape

```text
fabio [GLOBAL OPTIONS] <COMMAND GROUP> <SUBCOMMAND> [OPTIONS]
```

For live, version-specific help:

```bash
fabio --help
fabio <group> --help
fabio <group> <subcommand> --help
```

## Machine-readable discovery

```bash
fabio context agent
fabio context agent --group <group>
fabio context describe <group>
```

Continue with [global flags](./global-flags/) or select a command group in the sidebar.
