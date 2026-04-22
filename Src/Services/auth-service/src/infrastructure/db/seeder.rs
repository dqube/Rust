//! Idempotent seeder for built-in roles and baseline permissions.
//!
//! Runs at every service start. Uses deterministic role IDs so repeated
//! seeding upserts the same rows instead of creating duplicates.

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::info;
use uuid::Uuid;

use crate::domain::entities::{Role, RolePermission};
use crate::domain::ids::RoleId;
use crate::domain::repositories::{RolePermissionRepository, RoleRepository};
use crate::infrastructure::db::repositories::{PgRolePermissionRepository, PgRoleRepository};

/// Deterministic role UUIDs so seeding is idempotent across deployments.
const ROLE_ID_ADMIN: Uuid = Uuid::from_u128(0x0000_0000_0000_0000_a0a0_a0a0_a0a0_a0a0);
const ROLE_ID_CUSTOMER: Uuid = Uuid::from_u128(0x0000_0000_0000_0000_c0c0_c0c0_c0c0_c0c0);
const ROLE_ID_EMPLOYEE: Uuid = Uuid::from_u128(0x0000_0000_0000_0000_e0e0_e0e0_e0e0_e0e0);
const ROLE_ID_SUPPLIER: Uuid = Uuid::from_u128(0x0000_0000_0000_0000_5055_5055_5055_5055);

pub async fn run_seeder(db: &Arc<DatabaseConnection>) {
    let role_repo = PgRoleRepository(db.clone());
    let perm_repo = PgRolePermissionRepository(db.clone());

    let roles = [
        (ROLE_ID_ADMIN, "admin", "Full administrative access."),
        (ROLE_ID_CUSTOMER, "customer", "Standard customer account."),
        (ROLE_ID_EMPLOYEE, "employee", "Internal staff account."),
        (ROLE_ID_SUPPLIER, "supplier", "External supplier account."),
    ];

    for (id, name, desc) in roles {
        let role = Role::builtin(RoleId::from_uuid(id), name, desc);
        if let Err(e) = role_repo.save(&role).await {
            tracing::warn!(role = name, error = %e, "failed to seed role");
        }
    }

    // Baseline permissions — extend as business capabilities are added.
    let seed_perms: &[(Uuid, &str)] = &[
        (ROLE_ID_ADMIN, "users.read"),
        (ROLE_ID_ADMIN, "users.write"),
        (ROLE_ID_ADMIN, "roles.read"),
        (ROLE_ID_ADMIN, "roles.write"),
        (ROLE_ID_ADMIN, "permissions.manage"),
        (ROLE_ID_CUSTOMER, "profile.read"),
        (ROLE_ID_CUSTOMER, "profile.write"),
        (ROLE_ID_EMPLOYEE, "profile.read"),
        (ROLE_ID_EMPLOYEE, "profile.write"),
        (ROLE_ID_EMPLOYEE, "orders.read"),
        (ROLE_ID_SUPPLIER, "profile.read"),
        (ROLE_ID_SUPPLIER, "profile.write"),
    ];

    for (role_uuid, permission) in seed_perms {
        let rp = match RolePermission::new(RoleId::from_uuid(*role_uuid), (*permission).to_owned()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(permission = *permission, error = %e, "invalid permission seed");
                continue;
            }
        };
        if let Err(e) = perm_repo.save(&rp).await {
            tracing::warn!(permission = *permission, error = %e, "failed to seed permission");
        }
    }

    info!("Auth seeder completed.");
}
