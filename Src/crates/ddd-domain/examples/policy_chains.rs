use ddd_domain::policy::{PolicyChain, PolicyViolation, ClosurePolicy};
use ddd_domain::policy; // for the policy! macro

struct Order {
    total_amount: f64,
    items_count: usize,
    status: String,
}

fn main() {
    let mut order = Order {
        total_amount: 5000.0,
        items_count: 0,
        status: "Draft".into(),
    };

    // Define policies using the macro
    let min_amount = policy!(|o: &mut Order| {
        if o.total_amount < 10.0 {
            Err(PolicyViolation::new("Order amount too low"))
        } else {
            Ok(())
        }
    });

    let max_amount = policy!(|o: &mut Order| {
        if o.total_amount > 1000.0 {
            Err(PolicyViolation::with_code("Order exceeds credit limit", "EXCEEDS_LIMIT"))
        } else {
            Ok(())
        }
    });

    let must_have_items = policy!(|o: &mut Order| {
        if o.items_count == 0 {
            Err(PolicyViolation::new("Order must have at least one item"))
        } else {
            Ok(())
        }
    });

    // 1. Using a short-circuiting chain
    let chain = PolicyChain::new()
        .with(min_amount)
        .with(max_amount)
        .with(must_have_items);

    println!("Applying policy chain (short-circuiting)...");
    match chain.apply(&mut order) {
        Ok(_) => println!("Order is valid"),
        Err(e) => println!("Validation failed: {} (code: {:?})", e.message, e.code),
    }

    // 2. Using apply_all to collect all violations
    println!("\nApplying all policies (collecting)...");
    match chain.apply_all(&mut order) {
        Ok(_) => println!("Order is valid"),
        Err(violations) => {
            println!("Order has {} violations:", violations.len());
            for v in violations {
                println!("  - {}", v);
            }
        }
    }
}
