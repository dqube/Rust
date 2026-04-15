//! Demonstrates the Specification pattern from `ddd-domain`.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-domain --example specifications
//! ```

use ddd_domain::{Specification, SpecificationExt};

struct Product {
    price: f64,
    in_stock: bool,
}

struct PriceBelow(f64);
impl Specification<Product> for PriceBelow {
    fn is_satisfied_by(&self, p: &Product) -> bool {
        p.price < self.0
    }
}

struct InStock;
impl Specification<Product> for InStock {
    fn is_satisfied_by(&self, p: &Product) -> bool {
        p.in_stock
    }
}

fn main() {
    let products = [
        Product { price: 9.99, in_stock: true },
        Product { price: 49.99, in_stock: false },
        Product { price: 19.99, in_stock: true },
        Product { price: 5.00, in_stock: true },
    ];

    // Affordable AND in stock
    let affordable_in_stock = PriceBelow(20.0).and(InStock);

    let matching: Vec<_> = products
        .iter()
        .filter(|p| affordable_in_stock.is_satisfied_by(p))
        .collect();

    println!("Products below $20 and in stock: {}", matching.len());
    for p in &matching {
        println!("  ${:.2}", p.price);
    }
}
