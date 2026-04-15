use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ModernStores Auth Service",
        version = "1.0.0",
        description = "Authentication, user management, roles and permissions API."
    ),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Authentication", description = "Login, Registration, Token management"),
        (name = "Account", description = "Self-service account management"),
        (name = "Users", description = "User management and roles assignment"),
        (name = "Roles", description = "Role and permission management")
    ),
    paths(
        auth_docs::health,
        auth_docs::login,
        auth_docs::register,
        auth_docs::refresh_token,
        auth_docs::logout,
        auth_docs::change_password,
        auth_docs::forgot_password,
        auth_docs::reset_password,
        auth_docs::get_profile,
        auth_docs::get_user_by_email,
        auth_docs::list_users,
        auth_docs::get_user_roles,
        auth_docs::assign_user_role,
        auth_docs::remove_user_role,
        auth_docs::change_password_admin,
        auth_docs::activate_user,
        auth_docs::deactivate_user,
        auth_docs::check_permission,
        auth_docs::list_roles,
        auth_docs::create_role,
        auth_docs::get_role_permissions,
        auth_docs::add_role_permission,
        auth_docs::remove_role_permission,
    ),
    components(schemas(
        auth_docs::LoginBody,
        auth_docs::RegisterBody,
        auth_docs::RefreshBody,
        auth_docs::LogoutBody,
        auth_docs::ChangePasswordBody,
        auth_docs::ForgotPasswordBody,
        auth_docs::ResetPasswordBody,
        auth_docs::ChangePasswordAdminBody,
        auth_docs::AssignRoleBody,
        auth_docs::AddPermissionBody,
        auth_docs::CreateRoleBody,
        auth_docs::CheckPermissionBody,
        auth_docs::UserListQuery,
        auth_docs::UserRoleItem,
        auth_docs::UserRolesResponse,
        auth_docs::RoleItem,
        auth_docs::RolesResponse,
        auth_docs::RolePermissionsResponse,
        auth_docs::EmptyResponse,
    ))
)]
pub struct ApiDoc;

mod auth_docs {
    use serde::{Deserialize, Serialize};
    use utoipa::{IntoParams, ToSchema};

    #[utoipa::path(get, path = "/health", responses((status = 200, description = "Service is healthy")), tag = "Health")]
    pub async fn health() {}

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct EmptyResponse {}

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct LoginBody {
        pub username_or_email: String,
        pub password: String,
        pub ip_address: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct RegisterBody {
        pub username: String,
        pub email: String,
        pub password: String,
        pub user_type: String,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
        pub phone: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct RefreshBody {
        pub refresh_token: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct LogoutBody {
        pub refresh_token: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct ChangePasswordBody {
        pub user_id: String,
        pub current_password: String,
        pub new_password: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct ForgotPasswordBody {
        pub email: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct ResetPasswordBody {
        pub token: String,
        pub new_password: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct ChangePasswordAdminBody {
        pub new_password: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct AssignRoleBody {
        pub role_id: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct AddPermissionBody {
        pub permission: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct CreateRoleBody {
        pub name: String,
        pub role_type: String,
        pub description: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct CheckPermissionBody {
        pub user_id: String,
        pub permission: String,
    }

    #[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
    #[into_params(parameter_in = Query)]
    pub struct UserListQuery {
        pub page: Option<i32>,
        pub page_size: Option<i32>,
        pub search: Option<String>,
        pub is_active: Option<bool>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct UserRoleItem {
        pub user_role_id: String,
        pub user_id: String,
        pub role_id: String,
        pub role_name: String,
        pub role_type: String,
        pub assigned_at: String,
        pub expires_at: String,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct UserRolesResponse {
        pub user_roles: Vec<UserRoleItem>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct RoleItem {
        pub role_id: String,
        pub name: String,
        pub role_type: String,
        pub description: String,
        pub is_active: bool,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct RolesResponse {
        pub roles: Vec<RoleItem>,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct RolePermissionsResponse {
        pub permissions: Vec<String>,
    }

    #[utoipa::path(post, path = "/auth/login", request_body = LoginBody, responses((status = 200, description = "Login successful")), tag = "Authentication")]
    pub async fn login() {}
    #[utoipa::path(post, path = "/auth/register", request_body = RegisterBody, responses((status = 201, description = "User registered")), tag = "Authentication")]
    pub async fn register() {}
    #[utoipa::path(post, path = "/auth/refresh", request_body = RefreshBody, responses((status = 200, description = "Token refreshed")), tag = "Authentication")]
    pub async fn refresh_token() {}
    #[utoipa::path(post, path = "/auth/logout", request_body = LogoutBody, responses((status = 200, description = "Logged out")), tag = "Authentication")]
    pub async fn logout() {}
    #[utoipa::path(post, path = "/auth/change-password", request_body = ChangePasswordBody, responses((status = 200, description = "Password changed")), tag = "Account")]
    pub async fn change_password() {}
    #[utoipa::path(post, path = "/auth/forgot-password", request_body = ForgotPasswordBody, responses((status = 200, description = "Reset email sent")), tag = "Account")]
    pub async fn forgot_password() {}
    #[utoipa::path(post, path = "/auth/reset-password", request_body = ResetPasswordBody, responses((status = 200, description = "Password reset")), tag = "Account")]
    pub async fn reset_password() {}
    #[utoipa::path(get, path = "/auth/profile/{user_id}", params(("user_id" = String, Path, description = "User ID")), responses((status = 200, description = "User profile")), tag = "Users")]
    pub async fn get_profile() {}
    #[utoipa::path(get, path = "/users/email/{email}", params(("email" = String, Path, description = "User email")), responses((status = 200, description = "User details")), tag = "Users")]
    pub async fn get_user_by_email() {}
    #[utoipa::path(get, path = "/users", params(UserListQuery), responses((status = 200, description = "Paged list of users")), tag = "Users")]
    pub async fn list_users() {}
    #[utoipa::path(get, path = "/users/{user_id}/roles", params(("user_id" = String, Path, description = "User ID")), responses((status = 200, description = "Roles assigned to user", body = UserRolesResponse)), tag = "Users")]
    pub async fn get_user_roles() {}
    #[utoipa::path(post, path = "/users/{user_id}/roles", params(("user_id" = String, Path, description = "User ID")), request_body = AssignRoleBody, responses((status = 200, description = "Role assigned")), tag = "Users")]
    pub async fn assign_user_role() {}
    #[utoipa::path(delete, path = "/user-roles/{user_role_id}", params(("user_role_id" = String, Path, description = "User-role ID")), responses((status = 200, description = "Role removed")), tag = "Users")]
    pub async fn remove_user_role() {}
    #[utoipa::path(post, path = "/users/{user_id}/change-password-admin", params(("user_id" = String, Path, description = "User ID")), request_body = ChangePasswordAdminBody, responses((status = 200, description = "Password changed")), tag = "Users")]
    pub async fn change_password_admin() {}
    #[utoipa::path(post, path = "/users/{user_id}/activate", params(("user_id" = String, Path, description = "User ID")), responses((status = 200, description = "User activated")), tag = "Users")]
    pub async fn activate_user() {}
    #[utoipa::path(post, path = "/users/{user_id}/deactivate", params(("user_id" = String, Path, description = "User ID")), responses((status = 200, description = "User deactivated")), tag = "Users")]
    pub async fn deactivate_user() {}
    #[utoipa::path(post, path = "/auth/check-permission", request_body = CheckPermissionBody, responses((status = 200, description = "Permission check result")), tag = "Authentication")]
    pub async fn check_permission() {}
    #[utoipa::path(get, path = "/roles", responses((status = 200, description = "List all roles", body = RolesResponse)), tag = "Roles")]
    pub async fn list_roles() {}
    #[utoipa::path(post, path = "/roles", request_body = CreateRoleBody, responses((status = 201, description = "Role created", body = RoleItem)), tag = "Roles")]
    pub async fn create_role() {}
    #[utoipa::path(get, path = "/roles/{role_id}/permissions", params(("role_id" = String, Path, description = "Role ID")), responses((status = 200, description = "Permissions", body = RolePermissionsResponse)), tag = "Roles")]
    pub async fn get_role_permissions() {}
    #[utoipa::path(post, path = "/roles/{role_id}/permissions", params(("role_id" = String, Path, description = "Role ID")), request_body = AddPermissionBody, responses((status = 200, description = "Permission added")), tag = "Roles")]
    pub async fn add_role_permission() {}
    #[utoipa::path(delete, path = "/roles/{role_id}/permissions/{permission}", params(("role_id" = String, Path, description = "Role ID"),("permission" = String, Path, description = "Permission name")), responses((status = 200, description = "Permission removed")), tag = "Roles")]
    pub async fn remove_role_permission() {}
}
