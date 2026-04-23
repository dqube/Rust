use ddd_shared_kernel::declare_id;

declare_id!(ProductId);
declare_id!(ProductVariantId);
declare_id!(ProductImageId);
declare_id!(BrandId);
declare_id!(TaxConfigId);
declare_id!(PricingId);

/// Category uses a SERIAL (i32) auto-increment PK — not a UUID wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CategoryId(pub i32);

impl std::fmt::Display for CategoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
