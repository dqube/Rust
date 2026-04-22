//! gRPC server implementation for the Auth service.
//!
//! Translates tonic requests to mediator commands/queries, maps domain
//! entities to proto DTOs, and lifts [`AppError`] into [`tonic::Status`]
//! via [`ddd_api::grpc::error::GrpcErrorExt`].

use std::str::FromStr;
use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use tonic::{Request, Response, Status};

use crate::application::commands::{
    ActivateUser, AddRolePermission, AssignRole, ChangePassword, ChangePasswordAdmin, CreateRole,
    DeactivateUser, ForgotPassword, Login, LoginResult, Logout,
    RefreshToken as RefreshTokenCmd, Register, RemoveRolePermission, RemoveUserRole,
    ResetPassword,
};
use crate::application::queries::{
    CheckPermission, GetRolePermissionsById, GetRolePermissionsByNames, GetUser, GetUserByEmail,
    ListRoles, ListUserRoles, ListUsers, UserRoleWithName,
};
use crate::domain::entities::{Role, User};
use crate::domain::enums::UserType;
use crate::domain::ids::{RoleId, UserId, UserRoleId};
use crate::proto::{
    auth_service_server::{AuthService, AuthServiceServer},
    ActivateUserRequest, AddRolePermissionRequest, AssignRoleRequest, AssignRoleResponse,
    ChangePasswordAdminRequest, ChangePasswordRequest, CheckPermissionRequest,
    CheckPermissionResponse, CreateRoleRequest, DeactivateUserRequest, Empty,
    ForgotPasswordRequest, GetRolePermissionsByIdRequest, GetRolePermissionsRequest,
    GetUserByEmailRequest, GetUserRequest, GetUserResponse, ListRolesResponse,
    ListUserRolesRequest, ListUserRolesResponse, ListUsersRequest, ListUsersResponse,
    LoginRequest, LoginResponse, LogoutRequest, PermissionsResponse, RefreshTokenRequest,
    RegisterRequest, RegisterResponse, RemoveRolePermissionRequest, RemoveUserRoleRequest,
    ResetPasswordRequest, RoleInfo, UserInfo, UserRoleItem,
};

#[derive(Clone)]
pub struct AuthGrpcService {
    mediator: Arc<Mediator>,
}

impl AuthGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }
    pub fn into_server(self) -> AuthServiceServer<Self> {
        AuthServiceServer::new(self)
    }
}

// ── Mappers ─────────────────────────────────────────────────────────────────

fn fmt(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339()
}
fn fmt_opt(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
    dt.map(fmt).unwrap_or_default()
}

fn user_info_from_user(u: &User) -> UserInfo {
    UserInfo {
        user_id: u.id.to_string(),
        username: u.username.clone(),
        email: u.email.clone(),
        email_confirmed: u.email_confirmed,
        user_type: u.user_type.to_string(),
        two_factor_enabled: u.two_factor_enabled,
        is_active: u.is_active,
        is_locked: u.is_locked_out(),
        created_at: fmt(u.created_at),
    }
}

fn user_info_from_login(r: &LoginResult) -> UserInfo {
    UserInfo {
        user_id: r.user_id.to_string(),
        username: r.username.clone(),
        email: r.email.clone(),
        email_confirmed: r.email_confirmed,
        user_type: r.user_type.to_string(),
        two_factor_enabled: r.two_factor_enabled,
        is_active: r.is_active,
        is_locked: r.is_locked,
        created_at: fmt(r.created_at),
    }
}

fn role_info(r: &Role) -> RoleInfo {
    RoleInfo {
        role_id: r.id.to_string(),
        name: r.name.clone(),
        role_type: r.role_type.to_string(),
        description: r.description.clone().unwrap_or_default(),
        is_active: r.is_active,
        created_at: fmt(r.created_at),
    }
}

fn user_role_item(ur: &UserRoleWithName) -> UserRoleItem {
    UserRoleItem {
        user_role_id: ur.user_role.id.to_string(),
        user_id: ur.user_role.user_id.to_string(),
        role_id: ur.user_role.role_id.to_string(),
        role_name: ur.role_name.clone(),
        role_type: ur.role_type.clone(),
        assigned_at: fmt(ur.user_role.assigned_at),
        expires_at: fmt_opt(ur.user_role.expires_at),
    }
}

fn parse_user_id(s: &str, field: &str) -> Result<UserId, Status> {
    UserId::from_str(s).map_err(|e| Status::invalid_argument(format!("{field}: {e}")))
}
fn parse_role_id(s: &str, field: &str) -> Result<RoleId, Status> {
    RoleId::from_str(s).map_err(|e| Status::invalid_argument(format!("{field}: {e}")))
}
fn parse_user_role_id(s: &str, field: &str) -> Result<UserRoleId, Status> {
    UserRoleId::from_str(s).map_err(|e| Status::invalid_argument(format!("{field}: {e}")))
}
fn opt_user_id(s: &str, field: &str) -> Result<Option<UserId>, Status> {
    if s.is_empty() {
        Ok(None)
    } else {
        parse_user_id(s, field).map(Some)
    }
}
fn opt_phone(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

fn login_response_from(r: LoginResult) -> LoginResponse {
    let user = user_info_from_login(&r);
    LoginResponse {
        access_token: r.tokens.access_token,
        refresh_token: r.tokens.refresh_token,
        expires_at: fmt(r.tokens.access_expires_at),
        user: Some(user),
    }
}

// ── Service impl ────────────────────────────────────────────────────────────

#[tonic::async_trait]
impl AuthService for AuthGrpcService {
    // ── Auth flows ──────────────────────────────────────────────────────

    async fn login(
        &self,
        req: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let r = req.into_inner();
        let result = self
            .mediator
            .send::<Login>(Login {
                username_or_email: r.username_or_email,
                password: r.password,
                ip_address: opt_phone(r.ip_address),
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(login_response_from(result)))
    }

    async fn register(
        &self,
        req: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let r = req.into_inner();
        let user_type = UserType::from_str(if r.user_type.is_empty() {
            "Customer"
        } else {
            &r.user_type
        })
        .map_grpc_status()?;

        let id = self
            .mediator
            .send::<Register>(Register {
                username: r.username,
                email: r.email,
                password: r.password,
                user_type,
                phone: opt_phone(r.phone),
            })
            .await
            .map_grpc_status()?;

        let user = self
            .mediator
            .query::<GetUser>(GetUser { user_id: id })
            .await
            .map_grpc_status()?
            .ok_or_else(|| Status::internal("user missing immediately after register"))?;

        Ok(Response::new(RegisterResponse {
            user: Some(user_info_from_user(&user)),
        }))
    }

    async fn refresh_token(
        &self,
        req: Request<RefreshTokenRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let r = req.into_inner();
        let result = self
            .mediator
            .send::<RefreshTokenCmd>(RefreshTokenCmd {
                refresh_token: r.refresh_token,
                ip_address: opt_phone(r.ip_address),
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(login_response_from(result)))
    }

    async fn logout(
        &self,
        req: Request<LogoutRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send::<Logout>(Logout {
                refresh_token: r.refresh_token,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn change_password(
        &self,
        req: Request<ChangePasswordRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        self.mediator
            .send::<ChangePassword>(ChangePassword {
                user_id,
                current_password: r.current_password,
                new_password: r.new_password,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    // ── User queries ────────────────────────────────────────────────────

    async fn get_user(
        &self,
        req: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let r = req.into_inner();
        let id = parse_user_id(&r.user_id, "user_id")?;
        let user = self
            .mediator
            .query::<GetUser>(GetUser { user_id: id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(match user {
            Some(u) => GetUserResponse {
                found: true,
                user: Some(user_info_from_user(&u)),
            },
            None => GetUserResponse {
                found: false,
                user: None,
            },
        }))
    }

    async fn get_user_by_email(
        &self,
        req: Request<GetUserByEmailRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let r = req.into_inner();
        let user = self
            .mediator
            .query::<GetUserByEmail>(GetUserByEmail { email: r.email })
            .await
            .map_grpc_status()?;
        Ok(Response::new(match user {
            Some(u) => GetUserResponse {
                found: true,
                user: Some(user_info_from_user(&u)),
            },
            None => GetUserResponse {
                found: false,
                user: None,
            },
        }))
    }

    async fn list_users(
        &self,
        req: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let r = req.into_inner();
        let search = if r.search.is_empty() {
            None
        } else {
            Some(r.search)
        };
        let page = self
            .mediator
            .query::<ListUsers>(ListUsers {
                page: if r.page == 0 { 1 } else { r.page },
                per_page: if r.per_page == 0 { 20 } else { r.per_page },
                search,
                is_active: r.has_is_active.then_some(r.is_active),
                is_locked: r.has_is_locked.then_some(r.is_locked),
            })
            .await
            .map_grpc_status()?;

        let items: Vec<UserInfo> = page.items().iter().map(user_info_from_user).collect();
        Ok(Response::new(ListUsersResponse {
            items,
            total: page.total(),
            page: page.page(),
            per_page: page.per_page(),
            total_pages: page.total_pages(),
        }))
    }

    // ── User administration ─────────────────────────────────────────────

    async fn change_password_admin(
        &self,
        req: Request<ChangePasswordAdminRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        self.mediator
            .send::<ChangePasswordAdmin>(ChangePasswordAdmin {
                user_id,
                new_password: r.new_password,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn forgot_password(
        &self,
        req: Request<ForgotPasswordRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send::<ForgotPassword>(ForgotPassword { email: r.email })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn reset_password(
        &self,
        req: Request<ResetPasswordRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send::<ResetPassword>(ResetPassword {
                token: r.token,
                new_password: r.new_password,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn activate_user(
        &self,
        req: Request<ActivateUserRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        self.mediator
            .send::<ActivateUser>(ActivateUser { user_id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn deactivate_user(
        &self,
        req: Request<DeactivateUserRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        self.mediator
            .send::<DeactivateUser>(DeactivateUser { user_id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    // ── Roles ───────────────────────────────────────────────────────────

    async fn list_roles(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let roles = self
            .mediator
            .query::<ListRoles>(ListRoles)
            .await
            .map_grpc_status()?;
        Ok(Response::new(ListRolesResponse {
            roles: roles.iter().map(role_info).collect(),
        }))
    }

    async fn create_role(
        &self,
        req: Request<CreateRoleRequest>,
    ) -> Result<Response<RoleInfo>, Status> {
        let r = req.into_inner();
        let description = if r.description.is_empty() {
            None
        } else {
            Some(r.description)
        };
        let id = self
            .mediator
            .send::<CreateRole>(CreateRole {
                name: r.name,
                description,
            })
            .await
            .map_grpc_status()?;
        let roles = self
            .mediator
            .query::<ListRoles>(ListRoles)
            .await
            .map_grpc_status()?;
        let role = roles
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| Status::internal("role missing immediately after create"))?;
        Ok(Response::new(role_info(&role)))
    }

    async fn list_user_roles(
        &self,
        req: Request<ListUserRolesRequest>,
    ) -> Result<Response<ListUserRolesResponse>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        let rows = self
            .mediator
            .query::<ListUserRoles>(ListUserRoles { user_id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(ListUserRolesResponse {
            user_roles: rows.iter().map(user_role_item).collect(),
        }))
    }

    async fn assign_role(
        &self,
        req: Request<AssignRoleRequest>,
    ) -> Result<Response<AssignRoleResponse>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        let role_id = parse_role_id(&r.role_id, "role_id")?;
        let assigned_by = opt_user_id(&r.assigned_by, "assigned_by")?;
        let id = self
            .mediator
            .send::<AssignRole>(AssignRole {
                user_id,
                role_id,
                assigned_by,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(AssignRoleResponse {
            user_role_id: id.to_string(),
        }))
    }

    async fn remove_user_role(
        &self,
        req: Request<RemoveUserRoleRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let id = parse_user_role_id(&r.user_role_id, "user_role_id")?;
        self.mediator
            .send::<RemoveUserRole>(RemoveUserRole { user_role_id: id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    // ── Role permissions ────────────────────────────────────────────────

    async fn add_role_permission(
        &self,
        req: Request<AddRolePermissionRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let role_id = parse_role_id(&r.role_id, "role_id")?;
        self.mediator
            .send::<AddRolePermission>(AddRolePermission {
                role_id,
                permission: r.permission,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn remove_role_permission(
        &self,
        req: Request<RemoveRolePermissionRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let role_id = parse_role_id(&r.role_id, "role_id")?;
        self.mediator
            .send::<RemoveRolePermission>(RemoveRolePermission {
                role_id,
                permission: r.permission,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(Empty {}))
    }

    async fn get_role_permissions_by_id(
        &self,
        req: Request<GetRolePermissionsByIdRequest>,
    ) -> Result<Response<PermissionsResponse>, Status> {
        let r = req.into_inner();
        let role_id = parse_role_id(&r.role_id, "role_id")?;
        let perms = self
            .mediator
            .query::<GetRolePermissionsById>(GetRolePermissionsById { role_id })
            .await
            .map_grpc_status()?;
        Ok(Response::new(PermissionsResponse { permissions: perms }))
    }

    async fn check_permission(
        &self,
        req: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status> {
        let r = req.into_inner();
        let user_id = parse_user_id(&r.user_id, "user_id")?;
        let ok = self
            .mediator
            .query::<CheckPermission>(CheckPermission {
                user_id,
                permission: r.permission,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(CheckPermissionResponse {
            has_permission: ok,
        }))
    }

    async fn get_role_permissions(
        &self,
        req: Request<GetRolePermissionsRequest>,
    ) -> Result<Response<PermissionsResponse>, Status> {
        let r = req.into_inner();
        let perms = self
            .mediator
            .query::<GetRolePermissionsByNames>(GetRolePermissionsByNames {
                role_names: r.role_names,
            })
            .await
            .map_grpc_status()?;
        Ok(Response::new(PermissionsResponse { permissions: perms }))
    }
}

