use ddd_bff::clients::GrpcClientPool;
use ddd_bff::config::ResilienceConfig;
use ddd_shared_kernel::AppResult;
use std::time::Duration;

#[tokio::main]
async fn main() -> AppResult<()> {
    // 1. Configure resilience parameters
    let resilience = ResilienceConfig {
        timeout: Duration::from_secs(5),
        max_retries: 3,
        initial_backoff_ms: 100,
        concurrency_limit: 100,
        rate_limit: 1000,
        // The pool uses these to wrap the raw tonic channels
        ..Default::default()
    };

    // 2. Option A: Bulk registration (common for most BFFs)
    println!("--- Option A: Bulk Registration ---");
    let services = [
        ("order-service", "http://orders:50051"),
        ("catalog-service", "http://catalog:50051"),
    ];
    let pool = GrpcClientPool::from_services(services, &resilience)?;
    println!("Pool initialized with {} services.", pool.len());

    // 3. Option B: Fine-grained builder (per-service tuning)
    println!("\n--- Option B: Fine-grained Builder ---");
    let mut tuned_resilience = resilience.clone();
    tuned_resilience.timeout = Duration::from_secs(30); // Long timeout for heavy service

    let tuned_pool = GrpcClientPool::builder()
        .add("heavy-report-service", "http://reports:50051", tuned_resilience)?
        .add("fast-ping-service", "http://ping:50051", resilience)?
        .build();

    // 4. Retrieving a channel for a gRPC client
    // The channel returned is a 'ResilientChannel' which implements tonic's Service trait
    let order_channel = pool.channel("order-service")?;
    println!("Channel for order-service retrieved.");
    
    // Usage: let client = OrderServiceClient::new(order_channel);
    // Any call via this client will benefit from the pool's resilience layers.

    Ok(())
}
