---
title: Global flags
description: Output, projection, pagination, profile, and safety options shared by Fabio commands.
---

Global flags apply across Fabio's command surface.

| Flag | Purpose |
| --- | --- |
| `--output json\|table\|plain` | Select structured JSON, a human-readable table, or plain values. |
| `--query <expression>` | Project output with a JMESPath expression. |
| `--quiet` | Suppress successful stdout while preserving errors. |
| `--dry-run` | Preview supported mutations without sending them. |
| `--limit <number>` | Bound client-side list results. |
| `--all` | Follow every pagination token. |
| `--continuation-token <token>` | Resume a paginated list operation. |
| `--profile <name>` | Apply saved defaults from a named profile. |
| `--wrap-untrusted` | Mark API-derived text as untrusted for agent consumers. |

Run `fabio --help` for the flags supported by your installed version.
