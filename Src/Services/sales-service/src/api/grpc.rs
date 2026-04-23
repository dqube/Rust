use std::str::FromStr;
use std::sync::Arc;

use ddd_api::grpc::GrpcErrorExt;
use ddd_application::Mediator;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::dtos::{
    AppliedDiscountDto, ReturnDetailDto, ReturnDto, SaleDetailDto, SaleDto,
};
use crate::application::queries::*;
use crate::domain::enums::{ReturnReason, SalesChannel};
use crate::domain::ids::{ReturnId, SaleDetailId, SaleId};
use crate::proto::sales_service_server::{SalesService, SalesServiceServer};
use crate::proto::*;

pub struct SalesGrpcService {
    mediator: Arc<Mediator>,
}

impl SalesGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> SalesServiceServer<Self> {
        SalesServiceServer::new(self)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid {label}: {s}")))
}

fn parse_opt_uuid(s: &str) -> Option<Uuid> {
    if s.is_empty() { None } else { Uuid::parse_str(s).ok() }
}

fn parse_sale_id(s: &str) -> Result<SaleId, Status> {
    Ok(SaleId(parse_uuid(s, "sale_id")?))
}

fn parse_return_id(s: &str) -> Result<ReturnId, Status> {
    Ok(ReturnId(parse_uuid(s, "return_id")?))
}

fn parse_sale_detail_id(s: &str) -> Result<SaleDetailId, Status> {
    Ok(SaleDetailId(parse_uuid(s, "sale_detail_id")?))
}

fn parse_decimal(s: &str) -> rust_decimal::Decimal {
    s.parse().unwrap_or_default()
}

fn parse_opt_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if s.is_empty() { return None; }
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

fn parse_channel(s: &str) -> SalesChannel {
    SalesChannel::from_str(s).unwrap()
}

fn parse_return_reason(s: &str) -> ReturnReason {
    ReturnReason::from_str(s).unwrap()
}

fn to_sale_detail_info(d: &SaleDetailDto) -> SaleDetailInfo {
    SaleDetailInfo {
        id:               d.id.to_string(),
        sale_id:          d.sale_id.to_string(),
        product_id:       d.product_id.to_string(),
        variant_id:       d.variant_id.map(|u| u.to_string()).unwrap_or_default(),
        quantity:         d.quantity,
        unit_price:       d.unit_price.to_string(),
        applied_discount: d.applied_discount.to_string(),
        tax_applied:      d.tax_applied.to_string(),
        line_total:       d.line_total.to_string(),
        created_at:       d.created_at.to_rfc3339(),
    }
}

fn to_applied_discount_info(d: &AppliedDiscountDto) -> AppliedDiscountInfo {
    AppliedDiscountInfo {
        id:              d.id.to_string(),
        sale_id:         d.sale_id.to_string(),
        sale_detail_id:  d.sale_detail_id.map(|u| u.to_string()).unwrap_or_default(),
        campaign_id:     d.campaign_id.to_string(),
        rule_id:         d.rule_id.to_string(),
        discount_amount: d.discount_amount.to_string(),
        created_at:      d.created_at.to_rfc3339(),
    }
}

fn to_sale_info(s: &SaleDto) -> SaleInfo {
    SaleInfo {
        sale_id:          s.id.to_string(),
        store_id:         s.store_id,
        employee_id:      s.employee_id.to_string(),
        customer_id:      s.customer_id.map(|u| u.to_string()).unwrap_or_default(),
        register_id:      s.register_id,
        receipt_number:   s.receipt_number.clone(),
        transaction_time: s.transaction_time.to_rfc3339(),
        sub_total:        s.sub_total.to_string(),
        discount_total:   s.discount_total.to_string(),
        tax_amount:       s.tax_amount.to_string(),
        total_amount:     s.total_amount.to_string(),
        channel:          s.channel.clone(),
        status:           s.status.clone(),
        created_at:       s.created_at.to_rfc3339(),
        details:          s.sale_details.iter().map(to_sale_detail_info).collect(),
        discounts:        s.applied_discounts.iter().map(to_applied_discount_info).collect(),
    }
}

fn to_return_detail_info(d: &ReturnDetailDto) -> ReturnDetailInfo {
    ReturnDetailInfo {
        id:         d.id.to_string(),
        return_id:  d.return_id.to_string(),
        product_id: d.product_id.to_string(),
        quantity:   d.quantity,
        reason:     d.reason.clone(),
        restock:    d.restock,
        created_at: d.created_at.to_rfc3339(),
    }
}

fn to_return_info(r: &ReturnDto) -> ReturnInfo {
    ReturnInfo {
        return_id:    r.id.to_string(),
        sale_id:      r.sale_id.to_string(),
        return_date:  r.return_date.to_rfc3339(),
        employee_id:  r.employee_id.to_string(),
        customer_id:  r.customer_id.map(|u| u.to_string()).unwrap_or_default(),
        total_refund: r.total_refund.to_string(),
        created_at:   r.created_at.to_rfc3339(),
        details:      r.return_details.iter().map(to_return_detail_info).collect(),
    }
}

// ── gRPC trait impl ───────────────────────────────────────────────────────────

#[tonic::async_trait]
impl SalesService for SalesGrpcService {
    // ── Sales ─────────────────────────────────────────────────────────────────

    async fn create_sale(
        &self,
        req: Request<CreateSaleRequest>,
    ) -> Result<Response<CreateSaleResponse>, Status> {
        let r = req.into_inner();
        let cmd = CreateSale {
            store_id:       r.store_id,
            employee_id:    parse_uuid(&r.employee_id, "employee_id")?,
            register_id:    r.register_id,
            receipt_number: r.receipt_number,
            customer_id:    parse_opt_uuid(&r.customer_id),
            channel:        parse_channel(&r.channel),
        };
        let sale = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateSaleResponse { sale: Some(to_sale_info(&sale)) }))
    }

    async fn add_sale_detail(
        &self,
        req: Request<AddSaleDetailRequest>,
    ) -> Result<Response<AddSaleDetailResponse>, Status> {
        let r = req.into_inner();
        let cmd = AddSaleDetail {
            sale_id:     parse_sale_id(&r.sale_id)?,
            product_id:  parse_uuid(&r.product_id, "product_id")?,
            variant_id:  parse_opt_uuid(&r.variant_id),
            quantity:    r.quantity,
            unit_price:  parse_decimal(&r.unit_price),
            tax_applied: parse_decimal(&r.tax_applied),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddSaleDetailResponse {}))
    }

    async fn update_sale_detail(
        &self,
        req: Request<UpdateSaleDetailRequest>,
    ) -> Result<Response<UpdateSaleDetailResponse>, Status> {
        let r = req.into_inner();
        let cmd = UpdateSaleDetail {
            sale_id:        parse_sale_id(&r.sale_id)?,
            sale_detail_id: parse_sale_detail_id(&r.sale_detail_id)?,
            quantity:       r.quantity,
            unit_price:     parse_decimal(&r.unit_price),
            tax_applied:    parse_decimal(&r.tax_applied),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UpdateSaleDetailResponse {}))
    }

    async fn remove_sale_detail(
        &self,
        req: Request<RemoveSaleDetailRequest>,
    ) -> Result<Response<RemoveSaleDetailResponse>, Status> {
        let r = req.into_inner();
        let cmd = RemoveSaleDetail {
            sale_id:        parse_sale_id(&r.sale_id)?,
            sale_detail_id: parse_sale_detail_id(&r.sale_detail_id)?,
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RemoveSaleDetailResponse {}))
    }

    async fn apply_discount(
        &self,
        req: Request<ApplyDiscountRequest>,
    ) -> Result<Response<ApplyDiscountResponse>, Status> {
        let r = req.into_inner();
        let cmd = ApplyDiscount {
            sale_id:         parse_sale_id(&r.sale_id)?,
            sale_detail_id:  if r.sale_detail_id.is_empty() { None } else { Some(parse_sale_detail_id(&r.sale_detail_id)?) },
            campaign_id:     parse_uuid(&r.campaign_id, "campaign_id")?,
            rule_id:         parse_uuid(&r.rule_id, "rule_id")?,
            discount_amount: parse_decimal(&r.discount_amount),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ApplyDiscountResponse {}))
    }

    async fn complete_sale(
        &self,
        req: Request<CompleteSaleRequest>,
    ) -> Result<Response<CompleteSaleResponse>, Status> {
        let sale_id = parse_sale_id(&req.into_inner().sale_id)?;
        self.mediator.send(CompleteSale { sale_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CompleteSaleResponse {}))
    }

    async fn cancel_sale(
        &self,
        req: Request<CancelSaleRequest>,
    ) -> Result<Response<CancelSaleResponse>, Status> {
        let r = req.into_inner();
        let cmd = CancelSale {
            sale_id: parse_sale_id(&r.sale_id)?,
            reason:  r.reason,
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CancelSaleResponse {}))
    }

    async fn get_sale_by_id(
        &self,
        req: Request<GetSaleByIdRequest>,
    ) -> Result<Response<GetSaleByIdResponse>, Status> {
        let sale_id = parse_sale_id(&req.into_inner().sale_id)?;
        let result  = self.mediator.query(GetSaleById { sale_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSaleByIdResponse {
            sale:  result.as_ref().map(to_sale_info),
            found: result.is_some(),
        }))
    }

    async fn get_sale_by_receipt(
        &self,
        req: Request<GetSaleByReceiptRequest>,
    ) -> Result<Response<GetSaleByReceiptResponse>, Status> {
        let receipt_number = req.into_inner().receipt_number;
        let result = self.mediator.query(GetSaleByReceipt { receipt_number }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSaleByReceiptResponse {
            sale:  result.as_ref().map(to_sale_info),
            found: result.is_some(),
        }))
    }

    async fn get_sales(
        &self,
        req: Request<GetSalesRequest>,
    ) -> Result<Response<GetSalesResponse>, Status> {
        let r = req.into_inner();
        let page      = if r.page == 0 { 1 } else { r.page };
        let page_size = if r.page_size == 0 { 20 } else { r.page_size };
        let status    = if r.status.is_empty() { None } else { Some(r.status) };
        let (items, total) = self.mediator.query(GetSales { page, page_size, status }).await
            .map_err(|e| e.to_grpc_status())?;
        let total_pages = if page_size > 0 { (total as i32 + page_size - 1) / page_size } else { 0 };
        Ok(Response::new(GetSalesResponse {
            items:       items.iter().map(to_sale_info).collect(),
            total_count: total as i32,
            page,
            page_size,
            total_pages,
        }))
    }

    async fn update_sale_status(
        &self,
        req: Request<UpdateSaleStatusRequest>,
    ) -> Result<Response<UpdateSaleStatusResponse>, Status> {
        let r = req.into_inner();
        let cmd = UpdateSaleStatus {
            sale_id: parse_sale_id(&r.sale_id)?,
            status:  r.status,
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UpdateSaleStatusResponse {}))
    }

    async fn get_sales_by_store(
        &self,
        req: Request<GetSalesByStoreRequest>,
    ) -> Result<Response<GetSalesByStoreResponse>, Status> {
        let r = req.into_inner();
        let items = self.mediator.query(GetSalesByStore {
            store_id:  r.store_id,
            from_date: parse_opt_datetime(&r.from_date),
            to_date:   parse_opt_datetime(&r.to_date),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSalesByStoreResponse {
            items: items.iter().map(to_sale_info).collect(),
        }))
    }

    async fn get_sales_by_employee(
        &self,
        req: Request<GetSalesByEmployeeRequest>,
    ) -> Result<Response<GetSalesByEmployeeResponse>, Status> {
        let r = req.into_inner();
        let items = self.mediator.query(GetSalesByEmployee {
            employee_id: parse_uuid(&r.employee_id, "employee_id")?,
            from_date:   parse_opt_datetime(&r.from_date),
            to_date:     parse_opt_datetime(&r.to_date),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSalesByEmployeeResponse {
            items: items.iter().map(to_sale_info).collect(),
        }))
    }

    async fn get_sales_by_customer(
        &self,
        req: Request<GetSalesByCustomerRequest>,
    ) -> Result<Response<GetSalesByCustomerResponse>, Status> {
        let customer_id = parse_uuid(&req.into_inner().customer_id, "customer_id")?;
        let items = self.mediator.query(GetSalesByCustomer { customer_id }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSalesByCustomerResponse {
            items: items.iter().map(to_sale_info).collect(),
        }))
    }

    async fn get_sale_receipt_url(
        &self,
        req: Request<GetSaleReceiptUrlRequest>,
    ) -> Result<Response<GetSaleReceiptUrlResponse>, Status> {
        let sale_id = parse_sale_id(&req.into_inner().sale_id)?;
        let result  = self.mediator.query(GetSaleReceiptUrl { sale_id }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetSaleReceiptUrlResponse {
            url:         result.clone().unwrap_or_default(),
            has_receipt: result.is_some(),
        }))
    }

    async fn upload_sale_receipt(
        &self,
        req: Request<UploadSaleReceiptRequest>,
    ) -> Result<Response<UploadSaleReceiptResponse>, Status> {
        let r = req.into_inner();
        let cmd = UploadSaleReceipt {
            sale_id:      parse_sale_id(&r.sale_id)?,
            file_content: bytes::Bytes::from(r.file_content),
            file_name:    r.file_name,
            content_type: r.content_type,
        };
        let receipt_url = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UploadSaleReceiptResponse { receipt_url }))
    }

    // ── Returns ───────────────────────────────────────────────────────────────

    async fn create_return(
        &self,
        req: Request<CreateReturnRequest>,
    ) -> Result<Response<CreateReturnResponse>, Status> {
        let r = req.into_inner();
        let cmd = CreateReturn {
            sale_id:     parse_sale_id(&r.sale_id)?,
            employee_id: parse_uuid(&r.employee_id, "employee_id")?,
            customer_id: parse_opt_uuid(&r.customer_id),
        };
        let ret = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateReturnResponse { ret: Some(to_return_info(&ret)) }))
    }

    async fn add_return_detail(
        &self,
        req: Request<AddReturnDetailRequest>,
    ) -> Result<Response<AddReturnDetailResponse>, Status> {
        let r = req.into_inner();
        let cmd = AddReturnDetail {
            return_id:  parse_return_id(&r.return_id)?,
            product_id: parse_uuid(&r.product_id, "product_id")?,
            quantity:   r.quantity,
            reason:     parse_return_reason(&r.reason),
            restock:    r.restock,
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddReturnDetailResponse {}))
    }

    async fn process_return(
        &self,
        req: Request<ProcessReturnRequest>,
    ) -> Result<Response<ProcessReturnResponse>, Status> {
        let r = req.into_inner();
        let cmd = ProcessReturn {
            return_id:    parse_return_id(&r.return_id)?,
            total_refund: parse_decimal(&r.total_refund),
        };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ProcessReturnResponse {}))
    }

    async fn get_return_by_id(
        &self,
        req: Request<GetReturnByIdRequest>,
    ) -> Result<Response<GetReturnByIdResponse>, Status> {
        let return_id = parse_return_id(&req.into_inner().return_id)?;
        let result    = self.mediator.query(GetReturnById { return_id }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetReturnByIdResponse {
            ret:   result.as_ref().map(to_return_info),
            found: result.is_some(),
        }))
    }

    async fn get_returns_by_sale(
        &self,
        req: Request<GetReturnsBySaleRequest>,
    ) -> Result<Response<GetReturnsBySaleResponse>, Status> {
        let sale_id = parse_sale_id(&req.into_inner().sale_id)?;
        let returns = self.mediator.query(GetReturnsBySale { sale_id }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetReturnsBySaleResponse {
            returns: returns.iter().map(to_return_info).collect(),
        }))
    }

    async fn get_returns_by_employee(
        &self,
        req: Request<GetReturnsByEmployeeRequest>,
    ) -> Result<Response<GetReturnsByEmployeeResponse>, Status> {
        let r = req.into_inner();
        let returns = self.mediator.query(GetReturnsByEmployee {
            employee_id: parse_uuid(&r.employee_id, "employee_id")?,
            from_date:   parse_opt_datetime(&r.from_date),
            to_date:     parse_opt_datetime(&r.to_date),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetReturnsByEmployeeResponse {
            returns: returns.iter().map(to_return_info).collect(),
        }))
    }

    async fn get_returns_by_customer(
        &self,
        req: Request<GetReturnsByCustomerRequest>,
    ) -> Result<Response<GetReturnsByCustomerResponse>, Status> {
        let customer_id = parse_uuid(&req.into_inner().customer_id, "customer_id")?;
        let returns = self.mediator.query(GetReturnsByCustomer { customer_id }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetReturnsByCustomerResponse {
            returns: returns.iter().map(to_return_info).collect(),
        }))
    }
}
