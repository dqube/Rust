---
description: Review pending changes for DDD layering and mediator-pattern violations.
---

Review the current diff against repo architecture rules.

Steps:

1. Read `.claude/rules/ddd-architecture.md`.
2. Run `git diff` (staged + unstaged) and identify every file changed under `Src/crates/` and `Src/Services/`.
3. For each changed file, check:
   - **Layering**: imports do not violate the inward dependency rule. Flag any `use sea_orm::`, `use tonic::`, `use axum::`, `use async_nats::` appearing in `ddd-domain` or `ddd-application`.
   - **BFF layering**: flag any `use ddd_domain::` or `use ddd_application::` appearing in `ddd-bff`. Flag any `[[bin]]` or `fn main` in `ddd-bff/src/`.
   - **Mediator discipline**: handler registrations use `register_*_handler!` macros, not hand-written `inventory::submit!` or manual `mediator.builder()` wiring in `main.rs` (unless a test).
   - **Outbox vs. publish**: integration events must go through `OutboxRepository::append`, not `mediator.publish`. Flag any `mediator.publish` call whose event type implements `IntegrationEvent`.
   - **Port leakage**: repository trait signatures in `ddd-domain` do not reference SeaORM types.
   - **Naming**: new crates and packages use the `ddd-` prefix. Service binaries live under `Src/Services/`.
   - **axum route syntax**: routes use `{param}` syntax, not `:param`.
4. Output a punch list: ✅ clean items, ❌ violations with file:line and the rule they break.

Do not edit files. This command is read-only review.
