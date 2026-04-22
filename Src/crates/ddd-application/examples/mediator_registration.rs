use async_trait::async_trait;
use ddd_application::{
    Command, CommandHandler, Query, QueryHandler, Mediator,
    register_command_handler, register_query_handler
};
use ddd_shared_kernel::AppResult;
use uuid::Uuid;

// 1. Dependency Container
#[derive(Clone)]
struct AppDeps {
    pub region: String,
}

// 2. Command + Handler
struct PlaceOrder { sku: String }
impl Command for PlaceOrder { type Response = Uuid; }

struct PlaceOrderHandler { region: String }
#[async_trait]
impl CommandHandler<PlaceOrder> for PlaceOrderHandler {
    async fn handle(&self, cmd: PlaceOrder) -> AppResult<Uuid> {
        println!("Placing order for SKU {} in region {}", cmd.sku, self.region);
        Ok(Uuid::now_v7())
    }
}

// 3. Query + Handler
struct GetOrder { id: Uuid }
impl Query for GetOrder { type Response = String; }

struct GetOrderHandler;
#[async_trait]
impl QueryHandler<GetOrder> for GetOrderHandler {
    async fn handle(&self, q: GetOrder) -> AppResult<String> {
        Ok(format!("Order details for {}", q.id))
    }
}

// 4. Self-registration (Inventory mode)
// This macro submits the handler to the global registry at link time.
register_command_handler!(PlaceOrder, AppDeps, |deps: &AppDeps| {
    PlaceOrderHandler { region: deps.region.clone() }
});

register_query_handler!(GetOrder, AppDeps, |_deps: &AppDeps| {
    GetOrderHandler
});

#[tokio::main]
async fn main() -> AppResult<()> {
    let deps = AppDeps { region: "us-east-1".into() };

    // --- OPTION A: Inventory-based Discovery ---
    // Automatically finds all handlers registered with macros.
    println!("--- Using Inventory Discovery ---");
    let mediator = Mediator::from_inventory(&deps);
    
    let order_id = mediator.send(PlaceOrder { sku: "RUST-SHIRT-01".into() }).await?;
    let details = mediator.query(GetOrder { id: order_id }).await?;
    println!("Details: {}\n", details);

    // --- OPTION B: Manual Registration ---
    // Explicitly wire up handlers without using the global registry.
    println!("--- Using Manual Registration ---");
    let manual_mediator = Mediator::builder()
        .command::<PlaceOrder, _>(PlaceOrderHandler { region: "eu-west-1".into() })
        .query::<GetOrder, _>(GetOrderHandler)
        .build();

    let manual_id = manual_mediator.send(PlaceOrder { sku: "RUST-BOOK-02".into() }).await?;
    let manual_details = manual_mediator.query(GetOrder { id: manual_id }).await?;
    println!("Manual Details: {}", manual_details);

    Ok(())
}
