use std::sync::Arc;
use ddd_application::saga::{DefaultSagaOrchestrator, SagaDefinitionRegistry};
use ddd_shared_kernel::saga::{SagaDefinition, SagaStepDefinition, SagaOrchestrator, SagaInstanceRepository};
use ddd_shared_kernel::outbox::OutboxRepository;
use ddd_application::testing::{InMemorySagaInstanceRepository, InMemoryOutboxRepository};
use ddd_shared_kernel::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    // 1. Define the Saga
    let create_order_def = SagaDefinition {
        saga_type: "create_order_v1".into(),
        steps: vec![
            SagaStepDefinition {
                name: "reserve_inventory".into(),
                action_event_type: "inventory.reserve.v1".into(),
                action_subject: "cmd.inventory.reserve".into(),
                compensation_event_type: Some("inventory.release.v1".into()),
                compensation_subject: Some("cmd.inventory.release".into()),
            },
            SagaStepDefinition {
                name: "process_payment".into(),
                action_event_type: "payment.charge.v1".into(),
                action_subject: "cmd.payment.charge".into(),
                compensation_event_type: Some("payment.refund.v1".into()),
                compensation_subject: Some("cmd.payment.refund".into()),
            },
        ],
    };

    // 2. Setup Registry and Repositories (using In-Memory fakes for this example)
    let mut registry = SagaDefinitionRegistry::new();
    registry.register(create_order_def);

    let saga_repo = Arc::new(InMemorySagaInstanceRepository::default());
    let outbox_repo = Arc::new(InMemoryOutboxRepository::default());

    // 3. Initialize the Orchestrator
    let orchestrator = DefaultSagaOrchestrator::new(
        saga_repo.clone(),
        outbox_repo.clone(),
        registry,
    );

    // 4. Start a new Saga instance
    println!("Starting Saga...");
    let payload = serde_json::json!({
        "order_id": "ORD-001",
        "customer_id": "CUST-99",
        "amount": 150.0
    });
    let saga_id = orchestrator.start("create_order_v1", payload).await?;
    println!("Saga started with ID: {}", saga_id);

    // 5. Simulate first step completion
    // This would normally be triggered by an integration event handled in a consumer
    println!("\nSimulating Step 1 success...");
    orchestrator.on_step_completed(
        saga_id, 
        0, 
        serde_json::json!({"reservation_id": "RES-456"})
    ).await?;

    // 6. Simulate second step failure (triggering compensation)
    println!("\nSimulating Step 2 failure (insufficient funds)...");
    orchestrator.on_step_failed(
        saga_id, 
        1, 
        "insufficient funds".to_string()
    ).await?;

    // 7. Check final status
    let instance = saga_repo.find_by_id(saga_id).await?;
    println!("\nFinal Saga Status: {}", instance.status);
    println!("Current Step Index: {}", instance.current_step);
    
    // In a real app, the outbox would now contain the compensation command for Step 1
    let messages = outbox_repo.find_unpublished(10).await?;
    println!("\nOutbox contains {} pending messages.", messages.len());
    for msg in messages {
        println!("  - Subject: {}", msg.subject);
    }

    Ok(())
}
