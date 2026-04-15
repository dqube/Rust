use crate::domain::events::ProductId;

pub struct CreateProduct {
    pub sku: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: u32,
}
ddd_application::impl_command!(CreateProduct, ProductId);

pub struct UpdateStock {
    pub product_id: ProductId,
    pub stock: u32,
}
ddd_application::impl_command!(UpdateStock, ());

pub struct DeactivateProduct {
    pub product_id: ProductId,
}
ddd_application::impl_command!(DeactivateProduct, ());

pub struct RequestImageUploadUrl {
    pub product_id: ProductId,
    pub filename: String,
    pub content_type: String,
}
// Returns (upload_url, expires_in_secs).
ddd_application::impl_command!(RequestImageUploadUrl, (String, u32));

pub struct ConfirmImageUpload {
    pub product_id: ProductId,
    pub image_url: String,
}
ddd_application::impl_command!(ConfirmImageUpload, ());
