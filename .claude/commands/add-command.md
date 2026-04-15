---
description: Scaffold a new command, its handler, and inventory registration.
---

Add a new command to `ddd-application` following the repo's mediator pattern.

Arguments: $ARGUMENTS — expected form `<CommandName> <ResponseType>` (e.g. `CreateOrder uuid::Uuid`). If the response type is omitted, default to `()`.

Steps:

1. Read `.claude/rules/ddd-architecture.md` and `.claude/skills/ddd-building-blocks/SKILL.md`.
2. Pick the target file: a new `snake_case.rs` under `Src/crates/ddd-application/src/commands/` (create the `commands/` module and register it in `lib.rs` if it doesn't exist yet).
3. Emit:
   - The command struct.
   - `impl_command!(CommandName, ResponseType);`
   - A handler struct with explicit deps (from `AppDeps`).
   - `#[async_trait] impl CommandHandler<CommandName> for Handler { ... }` with a `todo!()` body and a `// TODO: business logic` comment.
   - `register_command_handler!(CommandName, AppDeps, |d: &AppDeps| Handler::new(...));`
4. Do **not** register from `main.rs` and do **not** add a new mediator.
5. Run `cargo check` in `Src/crates/ddd-application` and report.

If `AppDeps` does not yet exist in this repo, stop and ask the user where it should live (typically in the downstream service crate, not here). Do not invent one.
