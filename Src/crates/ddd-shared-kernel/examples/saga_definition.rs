use ddd_shared_kernel::saga::{SagaDefinition, SagaStepDefinition};

fn main() {
    // Define a "Create Order" Saga
    // Step 1: Reserve Inventory (with Release compensation)
    // Step 2: Authorize Payment (with Refund compensation)
    // Step 3: Ship Order (no compensation needed as it's the last step and irreversible here)

    let create_order_saga = SagaDefinition {
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
                name: "authorize_payment".into(),
                action_event_type: "payment.authorize.v1".into(),
                action_subject: "cmd.payment.authorize".into(),
                compensation_event_type: Some("payment.refund.v1".into()),
                compensation_subject: Some("cmd.payment.refund".into()),
            },
            SagaStepDefinition {
                name: "ship_order".into(),
                action_event_type: "shipping.ship.v1".into(),
                action_subject: "cmd.shipping.ship".into(),
                compensation_event_type: None,
                compensation_subject: None,
            },
        ],
    };

    println!("Saga Type: {}", create_order_saga.saga_type);
    for (i, step) in create_order_saga.steps.iter().enumerate() {
        println!("Step {}: {}", i + 1, step.name);
        println!("  Action: {} -> {}", step.action_event_type, step.action_subject);
        if let Some(comp) = &step.compensation_event_type {
            println!("  Compensation: {} -> {}", comp, step.compensation_subject.as_ref().unwrap());
        }
    }
}
