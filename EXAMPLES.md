<!-- Auto-generated from context data files. Do not edit manually. -->
<!-- Regenerate with: cargo test generate_examples_md -- --ignored -->

# Examples

> **AI agents**: Run `fabio context examples <group> <command>` to get machine-readable output examples with JMESPath queries. Run `fabio context workflow <name>` for multi-step recipes.

## Output Formats & Filtering

```bash
# JSON (default) -- structured envelope for agents
fabio workspace list
# {"data":[...],"count":5}

# Table -- human-readable columns
fabio workspace list -o table

# Plain -- one value per line, great for shell scripting
fabio workspace list -o plain

# CSV/TSV -- tabular export
fabio workspace list -o csv
fabio workspace list -o tsv

# JMESPath field projection
fabio workspace list --query 'data[].{name: displayName, id: id}'

# Limit results
fabio workspace list --limit 3

# Fetch all pages automatically
fabio item list --workspace $WS --all

# Dry-run a mutation
fabio workspace delete --id $WS --dry-run
```

## Workflow Recipes

Use `fabio context workflow <name>` for full step-by-step details.

### Lakehouse ETL Pipeline

```bash
fabio context workflow lakehouse-etl
```

### Real-Time Intelligence Pipeline

```bash
fabio context workflow rti-pipeline
```

### Direct Lake Report

```bash
fabio context workflow direct-lake-report
```

### CI/CD Deployment

```bash
fabio context workflow cicd-deploy
```

### Data Agent Setup

```bash
fabio context workflow data-agent-setup
```

## Common Patterns

### Workspace Management

```bash
fabio workspace list-folders --workspace $WS
```

### Lakehouse Operations

```bash
fabio lakehouse list --workspace $WS
```

### Notebook Operations

```bash
fabio notebook list --workspace $WS
```

### CI/CD Deployment

```bash
fabio deploy plan --workspace $WS
```

### Semantic Models

```bash
fabio semantic-model list --workspace $WS
```

### KQL Queries

```bash
fabio kql-database list --workspace $WS
```

### Data Agent

```bash
fabio data-agent list --workspace $WS
```

### Git Integration

```bash
fabio git status --workspace $WS
```

## Best Practices

Use `fabio context best-practices <topic>` for detailed guidance on:

- **throttling** -- Automatic 429 retry, bounded parallelism
- **lro** -- Long-running operation polling, `--wait` for jobs
- **pagination** -- `--all`, `--limit`, `--continuation-token`
- **admin-apis** -- When to use admin vs workspace-scoped commands
- **shortcuts** -- ADLS Gen2 connection + shortcut two-step pattern

## Critical API Behaviors

These non-obvious behaviors cause silent failures if ignored:

1. **PascalCase values required** -- `Overwrite` not `overwrite`, `Csv` not `csv`
2. **Workspace-scoped vs tenant-scoped** -- `deployment-pipeline`, `connection`, `domain` have no `--workspace`
3. **LRO awareness** -- Create/definition operations return 202; use `--wait` for jobs
4. **Token sharing** -- Same Fabric token works for both `api.fabric.microsoft.com` and `api.powerbi.com`
5. **Load-table format** -- Only `Csv` and `Parquet` supported (not JSON)
6. **Hard delete** -- 38 item types support `--hard-delete` to skip recycle bin
7. **Notebook source format** -- Cell `source` must be a list of strings, not a single string
8. **KQL Database query URI** -- Scoped to `{kusto_uri}/.default`, not the standard Fabric scope

---

*8 examples extracted from commands.json. Auto-generated from context data files.*
