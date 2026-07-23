---
title: Work with output and pipes
description: Select Fabio output formats, project JSON, and compose commands safely.
---

Fabio writes successful data to stdout and diagnostics to stderr.

## Choose an output format

JSON is the default:

```bash
fabio workspace list
```

For a person:

```bash
fabio workspace list --output table
```

For shell pipelines:

```bash
fabio workspace list --output plain --query "data[].id"
```

## Project with JMESPath

`--query` supports full JMESPath expressions:

```bash
fabio workspace list --query "data[?contains(displayName, 'Prod')].{id:id,name:displayName}"
```

Project early to reduce output size and agent token use.

## Bound results

Use `--limit` for predictable response sizes:

```bash
fabio item list --workspace <workspace-id> --limit 25
```

Use `--all` only when every page is required. A continuation token can resume pagination without repeating earlier requests.
