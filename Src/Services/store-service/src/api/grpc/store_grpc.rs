use std::sync::Arc;

use ddd_api::grpc::GrpcErrorExt;
use ddd_application::Mediator;
use rust_decimal::Decimal;
use tonic::{Request, Response, Status};

use crate::application::commands::*;
use crate::application::dtos::{RegisterDto, StoreDto};
use crate::application::queries::*;
use crate::domain::entities::StoreSchedule;
use crate::domain::enums::StoreStatus;
use crate::domain::ids::{RegisterId, StoreId};

pub mod proto {
    tonic::include_proto!("store");
}

use proto::store_grpc_server::{StoreGrpc, StoreGrpcServer};
use proto::*;

pub struct StoreGrpcService {
    mediator: Arc<Mediator>,
}

impl StoreGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> StoreGrpcServer<Self> {
        StoreGrpcServer::new(self)
    }
}

// ── Mappers ───────────────────────────────────────────────────────────────────

fn proto_schedule(s: &ScheduleInfo) -> StoreSchedule {
    StoreSchedule {
        day_of_week: s.day_of_week as u8,
        open_time:   if s.open_time.is_empty() { None } else { Some(s.open_time.clone()) },
        close_time:  if s.close_time.is_empty() { None } else { Some(s.close_time.clone()) },
        is_closed:   s.is_closed,
    }
}

fn dto_to_store_info(dto: &StoreDto) -> StoreInfo {
    StoreInfo {
        store_id:      dto.id,
        name:          dto.name.clone(),
        location_id:   dto.location_id,
        address:       Some(AddressInfo {
            street:      dto.address.street.clone(),
            city:        dto.address.city.clone(),
            postal_code: dto.address.postal_code.clone(),
            country:     dto.address.country.clone(),
        }),
        phone: dto.phone.clone(),
        geolocation: Some(GeolocationInfo {
            latitude:  dto.geolocation.latitude,
            longitude: dto.geolocation.longitude,
        }),
        schedules: dto.schedules.iter().map(|s| ScheduleInfo {
            day_of_week: s.day_of_week as i32,
            open_time:   s.open_time.clone().unwrap_or_default(),
            close_time:  s.close_time.clone().unwrap_or_default(),
            is_closed:   s.is_closed,
        }).collect(),
        services:       dto.services.clone(),
        status:         dto.status.clone(),
        is_operational: dto.is_operational,
        logo_url:       String::new(),
        created_at:     dto.created_at.to_rfc3339(),
        updated_at:     dto.updated_at.to_rfc3339(),
    }
}

fn dto_to_register_info(dto: &RegisterDto) -> RegisterInfo {
    RegisterInfo {
        register_id:     dto.id,
        store_id:        dto.store_id,
        name:            dto.name.clone(),
        current_balance: dto.current_balance.to_string(),
        status:          dto.status.clone(),
        is_open:         dto.is_open,
        last_open:       dto.last_open.map(|d| d.to_rfc3339()).unwrap_or_default(),
        last_close:      dto.last_close.map(|d| d.to_rfc3339()).unwrap_or_default(),
        created_at:      dto.created_at.to_rfc3339(),
        updated_at:      dto.updated_at.to_rfc3339(),
    }
}

fn parse_decimal(s: &str, field: &str) -> Result<Decimal, Status> {
    s.parse::<Decimal>().map_err(|_| Status::invalid_argument(format!("Invalid {field}: '{s}'")))
}

fn validate_range(value: f64, min: f64, max: f64, field: &str) -> Result<(), Status> {
    if value < min || value > max {
        Err(Status::invalid_argument(format!("{field} must be between {min} and {max}")))
    } else {
        Ok(())
    }
}

fn validate_non_negative(value: &Decimal, field: &str) -> Result<(), Status> {
    if value.is_sign_negative() {
        Err(Status::invalid_argument(format!("{field} must be non-negative")))
    } else {
        Ok(())
    }
}

fn validate_positive(value: &Decimal, field: &str) -> Result<(), Status> {
    if value <= &Decimal::ZERO {
        Err(Status::invalid_argument(format!("{field} must be greater than 0")))
    } else {
        Ok(())
    }
}

fn parse_store_status(s: &str) -> Result<StoreStatus, Status> {
    match s {
        "Active"      => Ok(StoreStatus::Active),
        "Inactive"    => Ok(StoreStatus::Inactive),
        "Maintenance" => Ok(StoreStatus::Maintenance),
        "Closed"      => Ok(StoreStatus::Closed),
        _ => Err(Status::invalid_argument("status must be one of Active, Inactive, Maintenance, Closed")),
    }
}

fn require_nonempty(s: &str, field: &str) -> Result<(), Status> {
    if s.is_empty() {
        Err(Status::invalid_argument(format!("{field} is required")))
    } else {
        Ok(())
    }
}

// ── Implementation ────────────────────────────────────────────────────────────

#[tonic::async_trait]
impl StoreGrpc for StoreGrpcService {

    async fn create_store(&self, req: Request<CreateStoreRequest>) -> Result<Response<StoreResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.name, "name")?;
        require_nonempty(&r.street, "street")?;
        require_nonempty(&r.city, "city")?;
        require_nonempty(&r.country, "country")?;
        require_nonempty(&r.phone, "phone")?;
        validate_range(r.latitude, -90.0, 90.0, "latitude")?;
        validate_range(r.longitude, -180.0, 180.0, "longitude")?;
        let cmd = CreateStore {
            name:                r.name,
            location_id:         r.location_id,
            address_street:      r.street,
            address_city:        r.city,
            address_postal_code: r.postal_code,
            address_country:     r.country,
            phone:               r.phone,
            geo_latitude:        r.latitude,
            geo_longitude:       r.longitude,
            schedules:           r.schedules.iter().map(proto_schedule).collect(),
            services:            r.services,
        };
        match self.mediator.send(cmd).await {
            Ok(dto) => Ok(Response::new(StoreResponse { found: true, store: Some(dto_to_store_info(&dto)) })),
            Err(e)  => Err(e.to_grpc_status()),
        }
    }

    async fn update_store_information(&self, req: Request<UpdateStoreInformationRequest>) -> Result<Response<StoreResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.name, "name")?;
        validate_range(r.latitude, -90.0, 90.0, "latitude")?;
        validate_range(r.longitude, -180.0, 180.0, "longitude")?;
        let cmd = UpdateStoreInformation {
            store_id:            StoreId(r.store_id),
            name:                r.name,
            address_street:      r.street,
            address_city:        r.city,
            address_postal_code: r.postal_code,
            address_country:     r.country,
            phone:               r.phone,
            geo_latitude:        r.latitude,
            geo_longitude:       r.longitude,
            schedules:           r.schedules.iter().map(proto_schedule).collect(),
            services:            r.services,
        };
        match self.mediator.send(cmd).await {
            Ok(dto) => Ok(Response::new(StoreResponse { found: true, store: Some(dto_to_store_info(&dto)) })),
            Err(e)  => Err(e.to_grpc_status()),
        }
    }

    async fn change_store_status(&self, req: Request<ChangeStoreStatusRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.status, "status")?;
        let cmd = ChangeStoreStatus { store_id: StoreId(r.store_id), status: parse_store_status(&r.status)? };
        self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn get_store(&self, req: Request<GetStoreRequest>) -> Result<Response<StoreResponse>, Status> {
        let r = req.into_inner();
        match self.mediator.query(GetStore { store_id: StoreId(r.store_id) }).await {
            Ok(Some(dto)) => Ok(Response::new(StoreResponse { found: true, store: Some(dto_to_store_info(&dto)) })),
            Ok(None)      => Ok(Response::new(StoreResponse { found: false, store: None })),
            Err(e)        => Err(e.to_grpc_status()),
        }
    }

    async fn list_stores(&self, req: Request<ListStoresRequest>) -> Result<Response<ListStoresResponse>, Status> {
        let r = req.into_inner();
        let status = if r.status.is_empty() { None } else { Some(StoreStatus::from_str(&r.status)) };
        let location_id = if r.location_id == 0 { None } else { Some(r.location_id) };
        let cmd = ListStores {
            page:        r.page,
            page_size:   r.page_size,
            search_term: if r.search_term.is_empty() { None } else { Some(r.search_term) },
            status,
            location_id,
        };
        match self.mediator.query(cmd).await {
            Ok(result) => {
                let total_count = result.total as i32;
                let page_size   = result.page_size;
                let total_pages = ((total_count + page_size - 1) / page_size).max(1);
                Ok(Response::new(ListStoresResponse {
                    items: result.items.iter().map(dto_to_store_info).collect(),
                    total_count, page: result.page, page_size, total_pages,
                }))
            }
            Err(e) => Err(e.to_grpc_status()),
        }
    }

    async fn get_store_logo_url(&self, req: Request<GetStoreLogoUrlRequest>) -> Result<Response<GetStoreLogoUrlResponse>, Status> {
        let r = req.into_inner();
        match self.mediator.query(GetStoreLogoUrl { store_id: StoreId(r.store_id) }).await {
            Ok(Some(url)) => Ok(Response::new(GetStoreLogoUrlResponse { found: true, url })),
            Ok(None)      => Ok(Response::new(GetStoreLogoUrlResponse { found: false, url: String::new() })),
            Err(e)        => Err(e.to_grpc_status()),
        }
    }

    async fn upload_store_logo(&self, req: Request<UploadStoreLogoRequest>) -> Result<Response<UploadStoreLogoResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.file_name, "file_name")?;
        require_nonempty(&r.content_type, "content_type")?;
        if r.file_content.is_empty() {
            return Err(Status::invalid_argument("file_content is required"));
        }
        let cmd = UploadStoreLogo {
            store_id:     StoreId(r.store_id),
            file_content: bytes::Bytes::from(r.file_content),
            file_name:    r.file_name,
            content_type: r.content_type,
        };
        match self.mediator.send(cmd).await {
            Ok(url) => Ok(Response::new(UploadStoreLogoResponse { logo_url: url })),
            Err(e)  => Err(e.to_grpc_status()),
        }
    }

    async fn delete_store_logo(&self, req: Request<DeleteStoreLogoRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        self.mediator.send(DeleteStoreLogo { store_id: StoreId(r.store_id) }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn create_register(&self, req: Request<CreateRegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.name, "name")?;
        let cmd = CreateRegister { store_id: StoreId(r.store_id), name: r.name };
        match self.mediator.send(cmd).await {
            Ok(dto) => Ok(Response::new(RegisterResponse { found: true, register: Some(dto_to_register_info(&dto)) })),
            Err(e)  => Err(e.to_grpc_status()),
        }
    }

    async fn open_register(&self, req: Request<OpenRegisterRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        let cash = parse_decimal(&r.starting_cash, "starting_cash")?;
        validate_non_negative(&cash, "starting_cash")?;
        self.mediator.send(OpenRegister { register_id: RegisterId(r.register_id), starting_cash: cash }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn close_register(&self, req: Request<CloseRegisterRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        let cash = parse_decimal(&r.ending_cash, "ending_cash")?;
        validate_non_negative(&cash, "ending_cash")?;
        self.mediator.send(CloseRegister { register_id: RegisterId(r.register_id), ending_cash: cash }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn add_cash_to_register(&self, req: Request<CashAmountRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        let amount = parse_decimal(&r.amount, "amount")?;
        validate_positive(&amount, "amount")?;
        self.mediator.send(AddCashToRegister { register_id: RegisterId(r.register_id), amount, note: r.note }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn remove_cash_from_register(&self, req: Request<CashAmountRequest>) -> Result<Response<EmptyResponse>, Status> {
        let r = req.into_inner();
        let amount = parse_decimal(&r.amount, "amount")?;
        validate_positive(&amount, "amount")?;
        self.mediator.send(RemoveCashFromRegister { register_id: RegisterId(r.register_id), amount, note: r.note }).await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn update_register_name(&self, req: Request<UpdateRegisterNameRequest>) -> Result<Response<RegisterResponse>, Status> {
        let r = req.into_inner();
        require_nonempty(&r.new_name, "new_name")?;
        let cmd = UpdateRegisterName { register_id: RegisterId(r.register_id), new_name: r.new_name };
        match self.mediator.send(cmd).await {
            Ok(dto) => Ok(Response::new(RegisterResponse { found: true, register: Some(dto_to_register_info(&dto)) })),
            Err(e)  => Err(e.to_grpc_status()),
        }
    }

    async fn get_register(&self, req: Request<GetRegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        let r = req.into_inner();
        match self.mediator.query(GetRegister { register_id: RegisterId(r.register_id) }).await {
            Ok(Some(dto)) => Ok(Response::new(RegisterResponse { found: true, register: Some(dto_to_register_info(&dto)) })),
            Ok(None)      => Ok(Response::new(RegisterResponse { found: false, register: None })),
            Err(e)        => Err(e.to_grpc_status()),
        }
    }

    async fn list_registers(&self, req: Request<ListRegistersRequest>) -> Result<Response<ListRegistersResponse>, Status> {
        let r = req.into_inner();
        if r.store_id <= 0 {
            return Err(Status::invalid_argument("store_id must be greater than 0"));
        }
        let cmd = ListRegisters { store_id: StoreId(r.store_id), page: r.page, page_size: r.page_size };
        match self.mediator.query(cmd).await {
            Ok(result) => {
                let total_count = result.total as i32;
                let page_size   = result.page_size;
                let total_pages = ((total_count + page_size - 1) / page_size).max(1);
                Ok(Response::new(ListRegistersResponse {
                    items: result.items.iter().map(dto_to_register_info).collect(),
                    total_count, page: result.page, page_size, total_pages,
                }))
            }
            Err(e) => Err(e.to_grpc_status()),
        }
    }
}
