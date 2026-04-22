use std::sync::Arc;

use ddd_api::grpc::ToGrpcStatus;
use ddd_application::Mediator;
use ddd_shared_kernel::PageRequest;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::entities::*;
use crate::domain::enums::*;
use crate::domain::ids::*;
use crate::proto::supplier_service_server::{SupplierService, SupplierServiceServer};
use crate::proto::*;

pub struct SupplierGrpcService {
    mediator: Arc<Mediator>,
}

impl SupplierGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> SupplierServiceServer<Self> {
        SupplierServiceServer::new(self)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_id(s: &str, label: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid {label}: {s}")))
}

fn parse_opt_id(s: &str) -> Option<Uuid> {
    if s.is_empty() { None } else { Uuid::parse_str(s).ok() }
}

fn parse_opt_decimal(s: &str) -> Option<rust_decimal::Decimal> {
    if s.is_empty() { None } else { s.parse().ok() }
}

fn parse_decimal(s: &str, label: &str) -> Result<rust_decimal::Decimal, Status> {
    s.parse().map_err(|_| Status::invalid_argument(format!("invalid decimal for {label}: {s}")))
}

fn parse_opt_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if s.is_empty() { return None; }
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&chrono::Utc))
}

fn opt_str(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

// ── Message converters ────────────────────────────────────────────────────────

fn to_supplier_message(s: Supplier) -> SupplierMessage {
    SupplierMessage {
        id:                        s.id.to_string(),
        user_id:                   s.user_id.map(|u| u.to_string()).unwrap_or_default(),
        supplier_code:             s.supplier_code,
        company_name:              s.company_name,
        tax_identification_number: s.tax_identification_number.unwrap_or_default(),
        registration_number:       s.registration_number.unwrap_or_default(),
        email:                     s.email.unwrap_or_default(),
        phone:                     s.phone.unwrap_or_default(),
        website:                   s.website.unwrap_or_default(),
        business_type:             s.business_type.unwrap_or_default(),
        years_in_business:         s.years_in_business.unwrap_or(0),
        status:                    s.status.to_string(),
        onboarding_status:         s.onboarding_status.to_string(),
        rating:                    s.rating.map(|d| d.to_string()).unwrap_or_default(),
        total_orders:              s.total_orders,
        notes:                     s.notes.unwrap_or_default(),
        created_at:                s.created_at.to_rfc3339(),
        created_by:                s.created_by.unwrap_or_default(),
        updated_at:                s.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        updated_by:                s.updated_by.unwrap_or_default(),
    }
}

fn to_address_message(a: SupplierAddress) -> SupplierAddressMessage {
    SupplierAddressMessage {
        id:           a.id.to_string(),
        supplier_id:  a.supplier_id.to_string(),
        address_type: a.address_type.to_string(),
        line1:        a.line1,
        line2:        a.line2.unwrap_or_default(),
        city:         a.city,
        state:        a.state.unwrap_or_default(),
        postal_code:  a.postal_code,
        country:      a.country,
        is_primary:   a.is_primary,
        notes:        a.notes.unwrap_or_default(),
        created_at:   a.created_at.to_rfc3339(),
        updated_at:   a.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_contact_message(c: SupplierContact) -> SupplierContactMessage {
    SupplierContactMessage {
        id:           c.id.to_string(),
        supplier_id:  c.supplier_id.to_string(),
        user_id:      c.user_id.map(|u| u.to_string()).unwrap_or_default(),
        contact_type: c.contact_type.to_string(),
        first_name:   c.first_name,
        last_name:    c.last_name,
        email:        c.email.unwrap_or_default(),
        phone:        c.phone.unwrap_or_default(),
        mobile:       c.mobile.unwrap_or_default(),
        position:     c.position.unwrap_or_default(),
        department:   c.department.unwrap_or_default(),
        is_primary:   c.is_primary,
        can_login:    c.can_login,
        notes:        c.notes.unwrap_or_default(),
        created_at:   c.created_at.to_rfc3339(),
        updated_at:   c.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_document_message(d: SupplierDocument, url: String, url_expires_at: String) -> SupplierDocumentMessage {
    SupplierDocumentMessage {
        id:             d.id.to_string(),
        supplier_id:    d.supplier_id.to_string(),
        file_name:      d.file_name,
        object_name:    d.object_name,
        content_type:   d.content_type,
        document_type:  d.document_type.unwrap_or_default(),
        created_at:     d.created_at.to_rfc3339(),
        created_by:     d.created_by.unwrap_or_default(),
        url,
        url_expires_at,
    }
}

fn to_product_message(p: SupplierProduct) -> SupplierProductMessage {
    SupplierProductMessage {
        id:                 p.id.to_string(),
        supplier_id:        p.supplier_id.to_string(),
        product_id:         p.product_id.to_string(),
        variant_id:         p.variant_id.map(|u| u.to_string()).unwrap_or_default(),
        supplier_sku:       p.supplier_sku.unwrap_or_default(),
        unit_cost:          p.unit_cost.to_string(),
        lead_time_days:     p.lead_time_days.unwrap_or(0),
        min_order_quantity: p.min_order_quantity.unwrap_or(0),
        is_preferred:       p.is_preferred,
        created_at:         p.created_at.to_rfc3339(),
        updated_at:         p.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_order_message(o: PurchaseOrder) -> PurchaseOrderMessage {
    PurchaseOrderMessage {
        id:                  o.id.to_string(),
        supplier_id:         o.supplier_id.to_string(),
        store_id:            o.store_id,
        order_date:          o.order_date.to_rfc3339(),
        expected_date:       o.expected_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
        status:              o.status.to_string(),
        total_amount:        o.total_amount.to_string(),
        shipping_address_id: o.shipping_address_id.map(|i| i.to_string()).unwrap_or_default(),
        contact_person_id:   o.contact_person_id.map(|i| i.to_string()).unwrap_or_default(),
        created_at:          o.created_at.to_rfc3339(),
        updated_at:          o.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        order_details:       o.order_details.into_iter().map(|d| PurchaseOrderDetailMessage {
            id:                d.id.to_string(),
            order_id:          d.order_id.to_string(),
            product_id:        d.product_id.to_string(),
            quantity:          d.quantity,
            unit_cost:         d.unit_cost.to_string(),
            received_quantity: d.received_quantity.unwrap_or(0),
            created_at:        d.created_at.to_rfc3339(),
        }).collect(),
    }
}

// ── gRPC trait impl ───────────────────────────────────────────────────────────

#[tonic::async_trait]
impl SupplierService for SupplierGrpcService {
    // ── Suppliers ─────────────────────────────────────────────────────────────

    async fn list_suppliers(
        &self, req: Request<ListSuppliersRequest>,
    ) -> Result<Response<ListSuppliersResponse>, Status> {
        let r = req.into_inner();
        let page     = if r.page == 0 { 1 } else { r.page };
        let per_page = if r.per_page == 0 { 20 } else { r.per_page };
        let result = self.mediator.query(ListSuppliers {
            active_only: r.active_only,
            search:      opt_str(&r.search),
            req:         PageRequest { page, per_page },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListSuppliersResponse {
            items:       result.items.into_iter().map(to_supplier_message).collect(),
            total:       result.total,
            page:        result.page,
            per_page:    result.per_page,
            total_pages: result.total_pages,
        }))
    }

    async fn create_supplier(
        &self, req: Request<CreateSupplierRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let cmd = CreateSupplier {
            company_name:              r.company_name,
            tax_identification_number: opt_str(&r.tax_identification_number),
            registration_number:       opt_str(&r.registration_number),
            email:                     opt_str(&r.email),
            phone:                     opt_str(&r.phone),
            website:                   opt_str(&r.website),
            business_type:             opt_str(&r.business_type),
            notes:                     opt_str(&r.notes),
            created_by:                opt_str(&r.created_by),
            address_type:   AddressType::from_str(&r.address_type),
            address_line1:  r.address_line1,
            address_city:   r.address_city,
            address_postal: r.address_postal,
            address_country: r.address_country,
            contact_type:       ContactType::Primary,
            contact_first_name: r.contact_first_name,
            contact_last_name:  r.contact_last_name,
            contact_email:      opt_str(&r.contact_email),
        };
        let s = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn get_supplier(
        &self, req: Request<GetSupplierRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let id = SupplierId(parse_id(&req.into_inner().id, "supplier_id")?);
        let s = self.mediator.query(GetSupplier { id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Supplier {id} not found")))?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn update_supplier(
        &self, req: Request<UpdateSupplierRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let cmd = UpdateSupplier {
            id:                        SupplierId(parse_id(&r.id, "supplier_id")?),
            company_name:              r.company_name,
            tax_identification_number: opt_str(&r.tax_identification_number),
            registration_number:       opt_str(&r.registration_number),
            email:                     opt_str(&r.email),
            phone:                     opt_str(&r.phone),
            website:                   opt_str(&r.website),
            business_type:             opt_str(&r.business_type),
            years_in_business:         if r.years_in_business == 0 { None } else { Some(r.years_in_business) },
            notes:                     opt_str(&r.notes),
            updated_by:                opt_str(&r.updated_by),
        };
        let s = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn activate_supplier(
        &self, req: Request<ActivateSupplierRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let s = self.mediator.send(ActivateSupplier {
            id: SupplierId(parse_id(&r.id, "supplier_id")?),
            updated_by: opt_str(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn deactivate_supplier(
        &self, req: Request<DeactivateSupplierRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let s = self.mediator.send(DeactivateSupplier {
            id: SupplierId(parse_id(&r.id, "supplier_id")?),
            updated_by: opt_str(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn delete_supplier(
        &self, req: Request<DeleteSupplierRequest>,
    ) -> Result<Response<Empty>, Status> {
        let id = SupplierId(parse_id(&req.into_inner().id, "supplier_id")?);
        self.mediator.send(DeleteSupplier { id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn update_supplier_status(
        &self, req: Request<UpdateSupplierStatusRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let s = self.mediator.send(UpdateSupplierStatus {
            id:         SupplierId(parse_id(&r.id, "supplier_id")?),
            status:     SupplierStatus::from_str(&r.status),
            updated_by: opt_str(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    async fn update_onboarding_status(
        &self, req: Request<UpdateOnboardingStatusRequest>,
    ) -> Result<Response<SupplierMessage>, Status> {
        let r = req.into_inner();
        let s = self.mediator.send(UpdateOnboardingStatus {
            id:                SupplierId(parse_id(&r.id, "supplier_id")?),
            onboarding_status: OnboardingStatus::from_str(&r.onboarding_status),
            updated_by:        opt_str(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_supplier_message(s)))
    }

    // ── Addresses ─────────────────────────────────────────────────────────────

    async fn get_supplier_addresses(
        &self, req: Request<GetSupplierAddressesRequest>,
    ) -> Result<Response<GetSupplierAddressesResponse>, Status> {
        let supplier_id = SupplierId(parse_id(&req.into_inner().supplier_id, "supplier_id")?);
        let addrs = self.mediator.query(GetSupplierAddresses { supplier_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSupplierAddressesResponse {
            items: addrs.into_iter().map(to_address_message).collect(),
        }))
    }

    // ── Contacts ──────────────────────────────────────────────────────────────

    async fn get_supplier_contacts(
        &self, req: Request<GetSupplierContactsRequest>,
    ) -> Result<Response<GetSupplierContactsResponse>, Status> {
        let supplier_id = SupplierId(parse_id(&req.into_inner().supplier_id, "supplier_id")?);
        let contacts = self.mediator.query(GetSupplierContacts { supplier_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSupplierContactsResponse {
            items: contacts.into_iter().map(to_contact_message).collect(),
        }))
    }

    async fn create_supplier_contact(
        &self, req: Request<CreateSupplierContactRequest>,
    ) -> Result<Response<SupplierContactMessage>, Status> {
        let r = req.into_inner();
        let cmd = CreateSupplierContact {
            supplier_id: SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            first_name:  r.first_name,
            last_name:   r.last_name,
            email:       opt_str(&r.email),
            phone:       opt_str(&r.phone),
            position:    opt_str(&r.position),
            is_primary:  r.is_primary,
            notes:       opt_str(&r.notes),
            created_by:  opt_str(&r.created_by),
        };
        let contact = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_contact_message(contact)))
    }

    // ── Documents ─────────────────────────────────────────────────────────────

    async fn get_supplier_documents(
        &self, req: Request<GetSupplierDocumentsRequest>,
    ) -> Result<Response<GetSupplierDocumentsResponse>, Status> {
        let supplier_id = SupplierId(parse_id(&req.into_inner().supplier_id, "supplier_id")?);
        let docs = self.mediator.query(GetSupplierDocuments { supplier_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSupplierDocumentsResponse {
            items: docs.into_iter().map(|(d, url, exp)| to_document_message(d, url, exp)).collect(),
        }))
    }

    async fn request_document_upload_url(
        &self, req: Request<RequestDocumentUploadUrlRequest>,
    ) -> Result<Response<RequestDocumentUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let cmd = RequestDocumentUploadUrl {
            supplier_id:   SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            file_name:     r.file_name,
            content_type:  r.content_type,
            document_type: opt_str(&r.document_type),
        };
        let (upload_url, object_name, expires_at) = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestDocumentUploadUrlResponse { upload_url, object_name, expires_at }))
    }

    async fn confirm_document_upload(
        &self, req: Request<ConfirmDocumentUploadRequest>,
    ) -> Result<Response<SupplierDocumentMessage>, Status> {
        let r = req.into_inner();
        let cmd = ConfirmDocumentUpload {
            supplier_id:   SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            object_name:   r.object_name,
            file_name:     r.file_name,
            content_type:  r.content_type,
            document_type: opt_str(&r.document_type),
            created_by:    opt_str(&r.created_by),
        };
        let doc = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_document_message(doc, String::new(), String::new())))
    }

    async fn delete_supplier_document(
        &self, req: Request<DeleteSupplierDocumentRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cmd = DeleteSupplierDocument {
            supplier_id: SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            document_id: DocumentId(parse_id(&r.document_id, "document_id")?),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Supplier Products ─────────────────────────────────────────────────────

    async fn list_supplier_products(
        &self, req: Request<ListSupplierProductsRequest>,
    ) -> Result<Response<ListSupplierProductsResponse>, Status> {
        let supplier_id = SupplierId(parse_id(&req.into_inner().supplier_id, "supplier_id")?);
        let products = self.mediator.query(ListSupplierProducts { supplier_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListSupplierProductsResponse {
            items: products.into_iter().map(to_product_message).collect(),
        }))
    }

    async fn add_supplier_product(
        &self, req: Request<AddSupplierProductRequest>,
    ) -> Result<Response<SupplierProductMessage>, Status> {
        let r = req.into_inner();
        let cmd = AddSupplierProduct {
            supplier_id:        SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            product_id:         parse_id(&r.product_id, "product_id")?,
            variant_id:         parse_opt_id(&r.variant_id),
            unit_cost:          parse_decimal(&r.unit_cost, "unit_cost")?,
            supplier_sku:       opt_str(&r.supplier_sku),
            lead_time_days:     if r.lead_time_days == 0 { None } else { Some(r.lead_time_days) },
            min_order_quantity: if r.min_order_quantity == 0 { None } else { Some(r.min_order_quantity) },
            is_preferred:       r.is_preferred,
            created_by:         opt_str(&r.created_by),
        };
        let product = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(product)))
    }

    async fn remove_supplier_product(
        &self, req: Request<RemoveSupplierProductRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cmd = RemoveSupplierProduct {
            supplier_id:         SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            supplier_product_id: SupplierProductId(parse_id(&r.supplier_product_id, "supplier_product_id")?),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Purchase Orders ───────────────────────────────────────────────────────

    async fn list_purchase_orders(
        &self, req: Request<ListPurchaseOrdersRequest>,
    ) -> Result<Response<ListPurchaseOrdersResponse>, Status> {
        let r = req.into_inner();
        let orders = self.mediator.query(ListPurchaseOrders {
            supplier_id: parse_opt_id(&r.supplier_id).map(SupplierId),
            store_id:    if r.store_id == 0 { None } else { Some(r.store_id) },
            status:      opt_str(&r.status),
            from_date:   parse_opt_datetime(&r.from_date),
            to_date:     parse_opt_datetime(&r.to_date),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListPurchaseOrdersResponse {
            items: orders.into_iter().map(to_order_message).collect(),
        }))
    }

    async fn create_purchase_order(
        &self, req: Request<CreatePurchaseOrderRequest>,
    ) -> Result<Response<PurchaseOrderMessage>, Status> {
        let r = req.into_inner();
        let mut details = Vec::with_capacity(r.order_details.len());
        for d in r.order_details {
            details.push(PurchaseOrderDetailInput {
                product_id: parse_id(&d.product_id, "product_id")?,
                quantity:   d.quantity,
                unit_cost:  parse_decimal(&d.unit_cost, "unit_cost")?,
            });
        }
        let cmd = CreatePurchaseOrder {
            supplier_id:         SupplierId(parse_id(&r.supplier_id, "supplier_id")?),
            store_id:            r.store_id,
            expected_date:       parse_opt_datetime(&r.expected_date),
            shipping_address_id: parse_opt_id(&r.shipping_address_id).map(AddressId),
            contact_person_id:   parse_opt_id(&r.contact_person_id).map(ContactId),
            created_by:          parse_opt_id(&r.created_by),
            order_details:       details,
        };
        let order = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_order_message(order)))
    }

    async fn get_purchase_order(
        &self, req: Request<GetPurchaseOrderRequest>,
    ) -> Result<Response<PurchaseOrderMessage>, Status> {
        let id = OrderId(parse_id(&req.into_inner().id, "order_id")?);
        let order = self.mediator.query(GetPurchaseOrder { id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("PurchaseOrder {id} not found")))?;
        Ok(Response::new(to_order_message(order)))
    }

    async fn submit_purchase_order(
        &self, req: Request<SubmitPurchaseOrderRequest>,
    ) -> Result<Response<PurchaseOrderMessage>, Status> {
        let r = req.into_inner();
        let order = self.mediator.send(SubmitPurchaseOrder {
            id:         OrderId(parse_id(&r.id, "order_id")?),
            updated_by: parse_opt_id(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_order_message(order)))
    }

    async fn cancel_purchase_order(
        &self, req: Request<CancelPurchaseOrderRequest>,
    ) -> Result<Response<PurchaseOrderMessage>, Status> {
        let r = req.into_inner();
        let order = self.mediator.send(CancelPurchaseOrder {
            id:         OrderId(parse_id(&r.id, "order_id")?),
            updated_by: parse_opt_id(&r.updated_by),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_order_message(order)))
    }
}
