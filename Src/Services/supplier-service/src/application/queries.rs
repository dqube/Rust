use chrono::{DateTime, Utc};
use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};

use crate::domain::entities::{
    PurchaseOrder, Supplier, SupplierAddress, SupplierContact, SupplierDocument, SupplierProduct,
};
use crate::domain::ids::*;

impl_query! { GetSupplier       { id: SupplierId }               -> Option<Supplier>           }

impl_query! {
    ListSuppliers {
        active_only: bool,
        search:      Option<String>,
        req:         PageRequest,
    } -> Page<Supplier>
}

impl_query! {
    GetSupplierAddresses { supplier_id: SupplierId } -> Vec<SupplierAddress>
}
impl_query! {
    GetSupplierContacts  { supplier_id: SupplierId } -> Vec<SupplierContact>
}
impl_query! {
    GetSupplierDocuments { supplier_id: SupplierId } -> Vec<(SupplierDocument, String, String)>
    // (document, url, url_expires_at)
}
impl_query! {
    ListSupplierProducts { supplier_id: SupplierId } -> Vec<SupplierProduct>
}

impl_query! { GetPurchaseOrder   { id: OrderId }                  -> Option<PurchaseOrder>      }

impl_query! {
    ListPurchaseOrders {
        supplier_id: Option<SupplierId>,
        store_id:    Option<i32>,
        status:      Option<String>,
        from_date:   Option<DateTime<Utc>>,
        to_date:     Option<DateTime<Utc>>,
    } -> Vec<PurchaseOrder>
}
