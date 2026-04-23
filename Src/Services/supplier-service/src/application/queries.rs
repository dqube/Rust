use chrono::{DateTime, Utc};
use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};

use crate::domain::entities::{
    PurchaseOrder, Supplier, SupplierAddress, SupplierContact, SupplierDocument, SupplierProduct,
};
use crate::domain::ids::*;

pub struct GetSupplier {
    pub id: SupplierId,
}
impl_query!(GetSupplier, Option<Supplier>);

pub struct ListSuppliers {
    pub active_only: bool,
    pub search:      Option<String>,
    pub req:         PageRequest,
}
impl_query!(ListSuppliers, Page<Supplier>);

pub struct GetSupplierAddresses {
    pub supplier_id: SupplierId,
}
impl_query!(GetSupplierAddresses, Vec<SupplierAddress>);

pub struct GetSupplierContacts {
    pub supplier_id: SupplierId,
}
impl_query!(GetSupplierContacts, Vec<SupplierContact>);

pub struct GetSupplierDocuments {
    pub supplier_id: SupplierId,
}
impl_query!(GetSupplierDocuments, Vec<(SupplierDocument, String, String)>);

pub struct ListSupplierProducts {
    pub supplier_id: SupplierId,
}
impl_query!(ListSupplierProducts, Vec<SupplierProduct>);

pub struct GetPurchaseOrder {
    pub id: OrderId,
}
impl_query!(GetPurchaseOrder, Option<PurchaseOrder>);

pub struct ListPurchaseOrders {
    pub supplier_id: Option<SupplierId>,
    pub store_id:    Option<i32>,
    pub status:      Option<String>,
    pub from_date:   Option<DateTime<Utc>>,
    pub to_date:     Option<DateTime<Utc>>,
}
impl_query!(ListPurchaseOrders, Vec<PurchaseOrder>);
