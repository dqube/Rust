//! Illustrates how `ddd-infrastructure` adapter types are composed.
//!
//! A live database / NATS connection is required for a fully-runnable example.
//! This file documents the setup pattern for reference.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-infrastructure --example adapter_overview
//! ```

fn main() {
    println!("ddd-infrastructure adapter overview");
    println!();
    println!("  db::   SeaORM repositories for outbox, inbox, idempotency, dead-letter, saga");
    println!("  messaging::  NATS publisher / subscriber + event serialization");
    println!("  telemetry::  OpenTelemetry + Prometheus setup");
    println!();
    println!("Adapters require a live Postgres / NATS instance at runtime.");
    println!("Use the in-memory fakes from ddd-application::testing for unit tests.");
}
