# Graphify-Assisted Service Migration Guide

How to use graphify graph data as token-efficient context when migrating a service to the DDD architecture.

## Why Graphify Saves Tokens

| Approach | Approx Tokens |
|---|---|
| Paste entire codebase | ~80k+ |
| Paste all service files | ~20k |
| Graphify community + god node files only | ~3–5k |

Graphify tells you **which files matter** via community membership and degree. You only paste the relevant subgraph, not everything.

## Step 1 — Run `graphify query` for targeted context

```bash
graphify query "<service-name> architecture migration to DDD"
```

BFS traversal from relevant nodes returns only the connected subgraph — ~500 tokens instead of the full 9627-edge graph.

## Step 2 — Extract the relevant communities from GRAPH_REPORT.md

Find the community containing the service being migrated:

```bash
grep -A 5 "<service-name>" graphify-out/GRAPH_REPORT.md
```

Find the reference service community (e.g. customer-service → `CustomerGrpcService`, 31 edges, Community 0/1).

## Step 3 — Minimal Claude prompt template

```
## Context from graphify (graph.json — 3423 nodes, 9627 edges)

God nodes in <reference-service> (reference pattern):
- <ReferenceGrpcService> (<N> edges) → [paste reference-service/src/api/grpc.rs]
- Pg<Reference>Repository (community X) → [paste reference-service/src/infrastructure/db/repositories.rs]

Service to migrate:
- [paste target-service/src/api/grpc.rs]
- [paste target-service/src/infrastructure/db/repositories.rs]

Community membership (from graphify):
- <ReferenceGrpcService> and <TargetGrpcService> are in Community N (cohesion 0.0X)
- Shared god nodes: parse_id() (50 edges), to_utc() (42 edges), audit() (42 edges)

Task: Migrate <target-service> to match <reference-service> DDD pattern.
Use ids from domain/ids.rs. Keep same god node call signatures.
```

## God Nodes to Always Include (shared across all services)

These are the highest-degree nodes in the workspace. Their signatures must match exactly in any migrated service.

| Node | Edges | File |
|---|---|---|
| `main()` | 79 | `src/main.rs` |
| `parse_id()` | 50 | `src/api/grpc.rs` |
| `parse_uuid()` | 50 | `src/api/grpc.rs` |
| `audit()` | 42 | `ddd-bff/src/middleware/audit.rs` |
| `to_utc()` | 42 | shared utility |
| `from_utc()` | 37 | shared utility |

## Migration Layer Order

Migrate highest-degree files first — errors surface early.

| Order | Layer | File | Why |
|---|---|---|---|
| 1 | Domain aggregate | `domain/entities/*.rs` | Everything depends on this |
| 2 | gRPC API | `api/grpc.rs` | God node — high degree |
| 3 | Command handlers | `application/handlers/*.rs` | Depend on domain |
| 4 | Integration events | `application/integration_events.rs` | Depend on commands |
| 5 | Commands / Queries | `application/commands.rs`, `queries.rs` | Mid-layer |
| 6 | Repository impl | `infrastructure/db/repositories.rs` | Last — depends on all above |

## Reference Services by Domain

| Domain | Reference Service | God Node | Edges |
|---|---|---|---|
| Catalog / Products | `catalog-service` | `CatalogGrpcService` | 44 |
| Auth / Users | `auth-service` | `AuthGrpcService` | — |
| Customers | `customer-service` | `CustomerGrpcService` | 31 |
| Shared lookups | `shared-service` | `SharedGrpcService` | 42 |
| Employees | `employee-service` | `EmployeeGrpcService` | — |

## Surprising Cross-Service Connections (from graphify)

Watch for these — they indicate hidden coupling that breaks DDD layering:

- `admin-bff/main.rs` → `PgCustomerRepository` (INFERRED) — BFF should not reference infra directly
- `ddd-api` interceptor → `ddd-bff` redaction — `ddd-api` must not depend on `ddd-bff`

## Regenerating the Graph After Migration

```bash
graphify update .   # incremental — only re-extracts changed files
```

Then verify the new service joined the correct community and its god nodes have the expected degree.
