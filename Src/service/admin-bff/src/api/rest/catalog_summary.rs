//! Aggregation handler — calls product-service gRPC to build a catalog summary.
//!
//! Fetches all products (paginated) and computes aggregate statistics:
//! total count, active/inactive split, price range, total stock.
//! This is the kind of cross-cutting query a BFF exists for — the downstream
//! service exposes simple CRUD, the BFF composes higher-level views.



use axum::extract::State;
use axum::Json;
use serde::Serialize;

use ddd_bff::prelude::*;

use crate::state::AppState;
use crate::proto;

/// Aggregated catalog summary.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CatalogSummary {
    /// Total number of products.
    pub total_products: u64,
    /// Number of active products.
    pub active_count: u64,
    /// Number of inactive products.
    pub inactive_count: u64,
    /// Lowest product price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<f64>,
    /// Highest product price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<f64>,
    /// Sum of stock across all products.
    pub total_stock: u64,
    /// The products themselves.
    pub products: Vec<proto::Product>,
}

/// `GET /admin/catalog/summary` — aggregated product catalog statistics.
///
/// Calls `ListProducts` via gRPC, then computes summary stats over the
/// result set.
#[utoipa::path(
    get,
    path = "/admin/catalog/summary",
    responses(
        (status = 200, description = "Catalog summary", body = CatalogSummary),
        (status = 500, description = "Product service unavailable", body = ProblemDetail),
    ),
    tag = "Aggregation"
)]
pub async fn get_catalog_summary(
    State(state): State<AppState>,
) -> Result<Json<CatalogSummary>, ProblemDetail> {
    // Fetch a large page to get the full catalog for aggregation.
    let mut client = state.product_client.client();
    let resp = client
        .list_products(proto::ListProductsRequest {
            page: 1,
            per_page: 1000,
        })
        .await
        .into_problem()?;

    let list = resp.into_inner();
    let products = &list.products;

    let active_count = products.iter().filter(|p| p.active).count() as u64;
    let inactive_count = products.iter().filter(|p| !p.active).count() as u64;
    let total_stock: u64 = products.iter().map(|p| p.stock as u64).sum();

    let min_price = products.iter().map(|p| p.price).reduce(f64::min);
    let max_price = products.iter().map(|p| p.price).reduce(f64::max);

    Ok(Json(CatalogSummary {
        total_products: list.total,
        active_count,
        inactive_count,
        min_price,
        max_price,
        total_stock,
        products: list.products,
    }))
}
