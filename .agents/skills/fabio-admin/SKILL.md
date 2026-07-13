---
name: fabio-admin
description: >-
  Intent-scoped fabio skill for Fabric tenant administration: capacity lifecycle, tenant-wide inventory and settings, domains, gateways, connections, managed private endpoints, and sensitivity labels. Most commands are tenant-scoped (no --workspace). Use for governance, connectivity, and capacity operations. Triggers: "capacity", "resume capacity", "suspend capacity", "tenant settings", "admin list workspaces", "domain", "gateway", "connection", "private endpoint", "sensitivity label".
license: MIT
---

# fabio-admin — Administration — capacity, tenant governance, connectivity, labels

> **Generated file — do not edit by hand.** This intent-scoped sub-skill of the `fabio` skill is generated from fabio's command schema plus authored judgment. Regenerate with `cargo test generate_subskills -- --ignored`. For install, auth, output envelope, global flags, and agent-safety rules, see the root `fabio` skill.

> **Prefer runtime introspection.** This index is a snapshot; the installed binary is always authoritative. Use `fabio context agent --group <group>` and `fabio context describe <group> <command>` for exact flags and output shapes.

## When to use
- Capacity lifecycle: list, resume, suspend, create, delete (ARM-scoped).
- Tenant-wide inventory and settings (requires Fabric admin role).
- Governance: domains (group workspaces), sensitivity labels.
- Connectivity: gateways, connections, managed private endpoints.

## When NOT to use (route elsewhere)
- Building or transforming data -> use the data-engineer persona.
- Workspace-scoped item CRUD -> use the specific workload skill.

## Command index

Generated from fabio's command schema. For full flag details use `fabio context agent --group <group>` or `fabio context describe <group> <command>`.

### fabio admin
Fabric tenant administration (settings, tags, workloads, users)

| Command | Mutates | Description |
|---|---|---|
| `fabio admin assign-domain-workspaces` | yes | Assign workspaces to a domain |
| `fabio admin assign-domain-workspaces-by-capacities` | yes | Assign workspaces to a domain by capacities |
| `fabio admin assign-domain-workspaces-by-principals` | yes | Assign workspaces to a domain by principals |
| `fabio admin bulk-assign-domain-roles` | yes | Bulk-assign roles to a domain |
| `fabio admin bulk-remove-labels` | yes | Bulk-remove sensitivity labels from items |
| `fabio admin bulk-remove-sharing-links` | yes | Bulk-remove sharing links |
| `fabio admin bulk-set-labels` | yes | Bulk-set sensitivity labels on items |
| `fabio admin bulk-unassign-domain-roles` | yes | Bulk-unassign roles from a domain |
| `fabio admin create-domain` | yes | Create a domain |
| `fabio admin create-tags` | yes | Bulk-create tags |
| `fabio admin create-workload-assignment` | yes | Create a workload assignment |
| `fabio admin delete-capacity-tenant-override` | yes | Delete a capacity delegated tenant setting override |
| `fabio admin delete-domain` | yes | Delete a domain |
| `fabio admin delete-tag` | yes | Delete a tag |
| `fabio admin delete-workload-assignment` | yes | Delete a workload assignment |
| `fabio admin grant-admin-access` | yes | Grant temporary admin access to a workspace |
| `fabio admin list-capacities-tenant-overrides` | no | List all capacities' delegated tenant setting overrides |
| `fabio admin list-capacity-tenant-overrides` | no | List delegated tenant setting overrides for a capacity |
| `fabio admin list-domain-role-assignments` | no | List role assignments for a domain |
| `fabio admin list-domain-workspaces` | no | List workspaces in a domain |
| `fabio admin list-domains` | no | List domains (admin view) |
| `fabio admin list-domains-tenant-overrides` | no | List all domains' delegated tenant setting overrides |
| `fabio admin list-external-data-shares` | no | List external data shares |
| `fabio admin list-git-connections` | no | List git connections across workspaces |
| `fabio admin list-item-users` | no | List users with access to an item (admin view) |
| `fabio admin list-items` | no | List items (admin view) |
| `fabio admin list-network-policies` | no | List network communication policies |
| `fabio admin list-tags` | no | List tags |
| `fabio admin list-tenant-settings` | no | List all tenant settings |
| `fabio admin list-user-access` | no | List access details for a user |
| `fabio admin list-workload-assignments` | no | List workload assignments |
| `fabio admin list-workloads` | no | List workloads |
| `fabio admin list-workspace-users` | no | List users in a workspace (admin view) |
| `fabio admin list-workspaces` | no | List workspaces (admin view) |
| `fabio admin list-workspaces-tenant-overrides` | no | List all workspaces' delegated tenant setting overrides |
| `fabio admin remove-admin-access` | yes | Remove temporary admin access from a workspace |
| `fabio admin remove-all-sharing-links` | yes | Remove all sharing links for specified items |
| `fabio admin restore-workspace` | yes | Restore a deleted workspace |
| `fabio admin revoke-external-data-share` | yes | Revoke an external data share |
| `fabio admin show-domain` | no | Show domain details |
| `fabio admin show-item` | no | Show item details (admin view) |
| `fabio admin show-workspace` | no | Show workspace details (admin view) |
| `fabio admin sync-domain-roles-to-subdomains` | yes | Sync domain role assignments to subdomains |
| `fabio admin unassign-all-domain-workspaces` | yes | Unassign all workspaces from a domain |
| `fabio admin unassign-domain-workspaces` | yes | Unassign workspaces from a domain |
| `fabio admin update-capacity-tenant-override` | yes | Update a capacity delegated tenant setting override |
| `fabio admin update-domain` | yes | Update a domain |
| `fabio admin update-tag` | yes | Update a tag |
| `fabio admin update-tenant-setting` | yes | Update a tenant setting |

### fabio capacity
List and inspect Fabric capacities

| Command | Mutates | Description |
|---|---|---|
| `fabio capacity check-name` | no | Check if a capacity name is available (ARM API) |
| `fabio capacity create` | yes | Create a new Fabric capacity (ARM API) |
| `fabio capacity delete` | yes | Delete a Fabric capacity (ARM API) |
| `fabio capacity list` | no | List capacities available to the caller (Fabric API) |
| `fabio capacity list-skus` | no | List available SKUs for Fabric capacities (ARM API) |
| `fabio capacity resume` | yes | Resume a suspended capacity (ARM API) |
| `fabio capacity show` | no | Show details of a specific capacity (Fabric API) |
| `fabio capacity suspend` | yes | Suspend (pause) a capacity (ARM API) |
| `fabio capacity update` | yes | Update an existing Fabric capacity (ARM API) |

### fabio domain
Manage domains (organize workspaces into business domains)

| Command | Mutates | Description |
|---|---|---|
| `fabio domain assign-by-capacity` | yes | Bulk-assign all workspaces by capacity to a domain |
| `fabio domain assign-by-principal` | yes | Bulk-assign all workspaces by principal to a domain |
| `fabio domain assign-workspaces` | yes | Assign workspaces to a domain |
| `fabio domain create` | yes | Create a new domain |
| `fabio domain delete` | yes | Delete a domain |
| `fabio domain list` | no | List domains in the tenant |
| `fabio domain list-workspaces` | no | List workspaces assigned to a domain |
| `fabio domain show` | no | Show details of a domain |
| `fabio domain unassign-workspaces` | yes | Unassign workspaces from a domain |
| `fabio domain update` | yes | Update domain properties |

### fabio gateway
Manage gateways (on-premises, `VNet`, members, role assignments)

| Command | Mutates | Description |
|---|---|---|
| `fabio gateway add-role-assignment` | yes | Add a role assignment to a gateway |
| `fabio gateway check-member-status` | no | Check the status of a gateway member (on-premises only) |
| `fabio gateway check-status` | no | Check the status of a gateway (`VNet` only) |
| `fabio gateway create` | yes | Create a new gateway (`VirtualNetwork` type) |
| `fabio gateway create-streaming` | yes | Create a new streaming virtual network gateway |
| `fabio gateway delete` | yes | Delete a gateway |
| `fabio gateway delete-member` | yes | Delete a gateway member |
| `fabio gateway delete-role-assignment` | yes | Delete a role assignment |
| `fabio gateway list` | no | List all gateways |
| `fabio gateway list-members` | no | List members of a gateway |
| `fabio gateway list-role-assignments` | no | List role assignments for a gateway |
| `fabio gateway restart` | yes | Restart a gateway (`VNet` only, LRO) |
| `fabio gateway show` | no | Show details of a gateway |
| `fabio gateway show-role-assignment` | no | Show a specific role assignment |
| `fabio gateway shutdown` | yes | Shut down a gateway (`VNet` only, LRO) |
| `fabio gateway update` | yes | Update gateway properties |
| `fabio gateway update-member` | yes | Update a gateway member |
| `fabio gateway update-role-assignment` | yes | Update a role assignment |

### fabio connection
Manage connections (cloud, on-premises, virtual network)

| Command | Mutates | Description |
|---|---|---|
| `fabio connection add-role-assignment` | yes | Add a role assignment to a connection |
| `fabio connection create` | yes | Create a new connection |
| `fabio connection delete` | yes | Delete a connection |
| `fabio connection delete-role-assignment` | yes | Delete a role assignment from a connection |
| `fabio connection list` | no | List all connections you have permission to access |
| `fabio connection list-role-assignments` | no | List role assignments for a connection |
| `fabio connection list-supported-types` | no | List supported connection types (gateway types catalog) |
| `fabio connection show` | no | Show details of a specific connection |
| `fabio connection show-role-assignment` | no | Show a specific role assignment for a connection |
| `fabio connection test-connection` | no | Test a connection (not supported for `StreamingVirtualNetworkGateway` connections) |
| `fabio connection update` | yes | Update a connection's name, credentials, or privacy level |
| `fabio connection update-role-assignment` | yes | Update a role assignment for a connection |

### fabio managed-private-endpoint
Manage workspace managed private endpoints

| Command | Mutates | Description |
|---|---|---|
| `fabio managed-private-endpoint create` | yes | Create a managed private endpoint |
| `fabio managed-private-endpoint delete` | yes | Delete a managed private endpoint |
| `fabio managed-private-endpoint list` | no | List managed private endpoints in a workspace |
| `fabio managed-private-endpoint show` | no | Show details of a managed private endpoint |

### fabio label
List and resolve sensitivity labels (from Microsoft Purview via Graph API)

| Command | Mutates | Description |
|---|---|---|
| `fabio label list` | no | List available sensitivity labels (from Microsoft Purview via Graph API) |

## Key gotchas
- Tenant-scoped commands have NO --workspace flag: capacity, connection, gateway, domain, deployment-pipeline, admin.
- Capacity suspend/resume/create/delete use the ARM scope (management.azure.com), not the Fabric scope.
- admin commands require a Fabric admin role (FORBIDDEN otherwise).
- label list resolves UUIDs to names via Microsoft Graph (needs M365 E5 + InformationProtection.Read).
- Prefer batch operations (workspace batch-assign-roles, domain batch-assign) to reduce throttling.

## Safety
- capacity suspend interrupts ALL running workloads (notebooks, pipelines, Spark jobs) on that capacity — warn about in-flight jobs.
- Tenant setting changes are broad — confirm scope before applying.
- Deleting a workspace is permanent and removes ALL items inside — warn and suggest --dry-run.

## See also
- fabio context persona fabric-admin
- fabio context best-practices admin-apis
- fabio context best-practices throttling
