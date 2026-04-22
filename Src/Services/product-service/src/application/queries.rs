use crate::domain::aggregate::Product;
use crate::domain::events::ProductId;
use ddd_shared_kernel::Page;

pub struct GetProduct {
    pub product_id: ProductId,
}
ddd_application::impl_query!(GetProduct, Option<Product>);

pub struct ListProducts {
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(ListProducts, Page<Product>);
