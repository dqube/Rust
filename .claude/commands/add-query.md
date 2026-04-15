---
description: Scaffold a new query, its handler, and inventory registration.
---

Add a new query to `ddd-application` using the mediator pattern.

Arguments: $ARGUMENTS — `<QueryName> <ResponseType>` (e.g. `GetOrder OrderDto`). Response type is required for queries.

Steps:

1. Read `.claude/rules/ddd-architecture.md` and `.claude/skills/ddd-building-blocks/SKILL.md`.
2. Create `snake_case.rs` under `Src/crates/ddd-application/src/queries/` (bootstrap the module if missing).
3. Emit the query struct, `impl_query!(QueryName, ResponseType);`, a handler with read-side deps from `AppDeps`, and `register_query_handler!(...)`.
4. Query handlers must not mutate state. If the response type is a domain entity, wrap it in a DTO at the boundary.
5. `cargo check` in `Src/crates/ddd-application` and report.
