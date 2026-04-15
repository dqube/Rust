---
description: Scaffold a new aggregate, its repository port, and a SeaORM skeleton.
---

Add a new aggregate to `ddd-domain` and a SeaORM repository skeleton to `ddd-infrastructure`.

Arguments: $ARGUMENTS — `<AggregateName>` (PascalCase, e.g. `Order`).

Steps:

1. Read `.claude/rules/ddd-architecture.md`.
2. In `Src/crates/ddd-domain/src/<snake>.rs`:
   - Define a `<Name>Id` with `declare_id!`.
   - Define the aggregate struct, constructor, and domain methods.
   - Implement `AggregateRoot`. Use `record_event!` to raise domain events inside methods.
   - Define the repository **trait** (port) in the same file or a sibling `repository.rs`. The trait must take/return domain types only — no SeaORM, no DTOs.
3. In `Src/crates/ddd-infrastructure/src/db/<snake>_repository.rs`:
   - Add a `SeaOrm<Name>Repository` skeleton implementing the port.
   - Map between the SeaORM model and the domain aggregate in a private `mod mapper`.
4. Do **not** leak SeaORM types into the port signature.
5. Run `cargo check` in both crates and report.
