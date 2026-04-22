use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use ddd_application::cqrs::{Command, CommandHandler};
use ddd_application::idempotency::{IdempotentCommand, IdempotentCommandHandler};
use ddd_application::testing::InMemoryIdempotencyStore;
use ddd_shared_kernel::AppResult;
use uuid::Uuid;

// 1. Define an Idempotent Command
#[derive(serde::Serialize, serde::Deserialize)]
struct CreateProduct {
    pub sku: String,
    pub idempotency_key: String,
}

impl Command for CreateProduct {
    type Response = Uuid;
}

impl IdempotentCommand for CreateProduct {
    fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
}

// 2. Define the Inner Handler
struct CreateProductHandler;

#[async_trait]
impl CommandHandler<CreateProduct> for CreateProductHandler {
    async fn handle(&self, cmd: CreateProduct) -> AppResult<Uuid> {
        println!("Inner handler executing for SKU: {}", cmd.sku);
        // Simulate some work
        Ok(Uuid::now_v7())
    }
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let store = Arc::new(InMemoryIdempotencyStore::new());
    let inner = Arc::new(CreateProductHandler);
    
    // 3. Wrap the inner handler with the Idempotency Decorator
    let handler = IdempotentCommandHandler::new(
        inner,
        store.clone(),
        Duration::from_secs(3600), // 1 hour TTL
    );

    let cmd = CreateProduct {
        sku: "RUST-PLUSH-01".into(),
        idempotency_key: "unique-request-id-123".into(),
    };

    // First call
    println!("--- First Call ---");
    let id1 = handler.handle(cmd).await?;
    println!("Product created with ID: {}", id1);

    // Second call with same idempotency key
    println!("\n--- Second Call (Duplicate) ---");
    let cmd_dup = CreateProduct {
        sku: "RUST-PLUSH-01-CHANGED".into(), // Content doesn't matter, key does
        idempotency_key: "unique-request-id-123".into(),
    };
    
    let id2 = handler.handle(cmd_dup).await?;
    println!("Returned ID from cache: {}", id2);
    
    assert_eq!(id1, id2, "Responses must be identical");
    println!("Idempotency check successful!");

    Ok(())
}
