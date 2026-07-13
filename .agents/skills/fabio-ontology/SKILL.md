---
name: fabio-ontology
description: >-
  Intent-scoped fabio skill for Fabric knowledge/graph modeling: ontology items (entity/relationship types and bindings), graph models, and graph querysets. Use to define/evolve ontologies and query graphs for agent grounding. fabio can also export a tenant scan as OWL (context tenant --format owl) and import it. Triggers: "ontology", "fabric iq ontology", "knowledge graph", "graph model", "graph query", "entity type", "relationship type", "owl".
license: MIT
---

# fabio-ontology — Ontology & Graph — Fabric IQ ontologies, graph models, graph querysets

> **Generated file — do not edit by hand.** This intent-scoped sub-skill of the `fabio` skill is generated from fabio's command schema plus authored judgment. Regenerate with `cargo test generate_subskills -- --ignored`. For install, auth, output envelope, global flags, and agent-safety rules, see the root `fabio` skill.

> **Prefer runtime introspection.** This index is a snapshot; the installed binary is always authoritative. Use `fabio context agent --group <group>` and `fabio context describe <group> <command>` for exact flags and output shapes.

## When to use
- Creating/evolving an ontology item (entity types, relationship types, data bindings).
- Managing graph models and running graph querysets.
- Grounding an agent in a knowledge graph over Fabric data.
- Importing an OWL schema (e.g. one produced by 'fabio context tenant --format owl').

## When NOT to use (route elsewhere)
- Relational T-SQL modeling -> use fabio-warehouse-sql.
- The Delta/lakehouse data the ontology binds to -> use fabio-lakehouse.
- Semantic (tabular) models for BI -> use fabio-bi.

## Command index

Generated from fabio's command schema. For full flag details use `fabio context agent --group <group>` or `fabio context describe <group> <command>`.

### fabio ontology
Manage ontologies (entity types, data bindings)

| Command | Mutates | Description |
|---|---|---|
| `fabio ontology create` | yes | Create an ontology |
| `fabio ontology delete` | yes | Delete an ontology |
| `fabio ontology export` | no | Export a Fabric Ontology to OWL format (RDF/XML or JSON-LD) |
| `fabio ontology get-definition` | no | Get the ontology definition (entity types, bindings) |
| `fabio ontology import` | yes | Import an OWL ontology (RDF/XML or JSON-LD) and convert to Fabric format |
| `fabio ontology list` | no | List ontologies in a workspace |
| `fabio ontology show` | no | Show details of an ontology |
| `fabio ontology update` | yes | Update ontology properties (name and/or description) |
| `fabio ontology update-definition` | yes | Update the ontology definition (replaces current definition) |

### fabio graph-model
Manage graph models (knowledge graph)

| Command | Mutates | Description |
|---|---|---|
| `fabio graph-model create` | yes | Create a new graph model |
| `fabio graph-model delete` | yes | Delete a graph model |
| `fabio graph-model execute-query` | no | Execute a graph query |
| `fabio graph-model get-definition` | no | Get the definition of a graph model |
| `fabio graph-model get-queryable-graph-type` | no | Get the queryable graph type |
| `fabio graph-model initialize` | yes | Initialize a graph model for querying (portal-only operation) |
| `fabio graph-model list` | no | List graph models in a workspace |
| `fabio graph-model refresh-graph` | yes | Trigger a graph refresh job |
| `fabio graph-model show` | no | Show details of a graph model |
| `fabio graph-model update` | yes | Update graph model properties (name and/or description) |
| `fabio graph-model update-definition` | yes | Update the definition of a graph model |

### fabio graph-query-set
Manage graph query sets

| Command | Mutates | Description |
|---|---|---|
| `fabio graph-query-set create` | yes | Create a new graph query set |
| `fabio graph-query-set delete` | yes | Delete a graph query set |
| `fabio graph-query-set get-definition` | no | Get the definition of a graph query set |
| `fabio graph-query-set list` | no | List graph query sets in a workspace |
| `fabio graph-query-set show` | no | Show details of a graph query set |
| `fabio graph-query-set update` | yes | Update graph query set properties |
| `fabio graph-query-set update-definition` | yes | Update the definition of a graph query set |

## Must / Prefer / Avoid
### MUST
- Define entity/relationship types before adding data bindings.
- Use the item-definition format for ontology create/update (see 'fabio context schema ontology').

### PREFER
- context tenant --format owl to bootstrap an ontology schema from a real workspace scan, then ontology import.
- Runtime introspection (context agent --group ontology|graph-model) for exact flags.

### AVOID
- Binding to data sources that do not yet exist — create the underlying items first.
- Confusing an ontology (knowledge graph schema) with a semantic model (BI tabular model).

## Key gotchas
- Ontology definitions use the item-definition (base64 parts) format; fetch the template with 'fabio context schema ontology'.
- fabio's context tenant graph can emit OWL/RDF that imports directly via 'fabio ontology import --file'.

## Troubleshooting
| Symptom | Fix |
|---|---|
| Ontology import rejected | Validate the OWL/JSON-LD against the ontology schema (context schema ontology); ensure entity types precede bindings. |
| Binding references a missing item | Create the bound data source (lakehouse/eventhouse/etc.) first, then add the binding. |

## Safety
- Overwriting an ontology definition replaces its type system and bindings — confirm with the user.

## Shared references
Cross-cutting operational guidance (the "common" layer) — consult the relevant topic before non-trivial work:

| Reference | Covers |
|---|---|
| `fabio context best-practices throttling` | fabio transparently handles 429 (Too Many Requests) and gateway errors. Agents do NOT need to implement retry logic. |
| `fabio context best-practices pagination` | fabio handles pagination via --all (auto-fetch all pages), --continuation-token (resume), and --limit (truncate). Agents rarely need to paginate manually. |
| `fabio context best-practices lro` | Many Fabric operations are async (return 202). fabio polls them automatically. Use --wait for job operations. |

## See also
- fabio context schema ontology
- fabio context persona data-engineer
