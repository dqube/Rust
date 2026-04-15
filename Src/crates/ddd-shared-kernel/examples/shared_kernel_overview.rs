//! Demonstrates the core types exported by `ddd-shared-kernel`.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-shared-kernel --example shared_kernel_overview
//! ```

use ddd_shared_kernel::{
    AppError, AppResult,
    pagination::{Page, PageRequest},
};

fn find_order(id: &str) -> AppResult<String> {
    if id == "123" {
        Ok(format!("Order #{id}"))
    } else {
        Err(AppError::not_found("Order", id))
    }
}

fn main() {
    // AppError / AppResult
    match find_order("123") {
        Ok(order) => println!("Found: {order}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    match find_order("999") {
        Ok(order) => println!("Found: {order}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    // Pagination
    let request = PageRequest::new(1, 5);
    println!("\nPageRequest: page={}, per_page={}, offset={}", request.page(), request.per_page(), request.offset());

    let items: Vec<i32> = (1..=5).collect();
    let page = Page::new(items, 42, request.page(), request.per_page());
    println!(
        "Page: {} items, total={}, has_next={}, has_previous={}",
        page.items().len(),
        page.total(),
        page.has_next(),
        page.has_previous()
    );
}
