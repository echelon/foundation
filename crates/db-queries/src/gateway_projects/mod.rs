use chrono::{DateTime, Utc};
use ids::{AccountId, GatewayPrincipalId, GatewayProjectId, GatewayProjectKeyId, UserId};

mod attached_principal;
mod create_principal;
mod create_project;
mod delete_grant;
mod ensure_default;
mod find_credential;
mod insert_key;
mod list_grants;
mod list_keys;
mod list_principals;
mod list_projects;
mod put_grant;
mod revoke_key;
mod revoke_principal;

pub use attached_principal::{attach_principal, attached_principal, detach_principal, lock_project_key};
pub use create_principal::{create_principal, NewPrincipal};
pub use create_project::create_project;
pub use delete_grant::delete_grant;
pub use ensure_default::ensure_default;
pub use find_credential::find_credential;
pub use insert_key::{insert_key, NewProjectKey};
pub use list_grants::list_grants;
pub use list_keys::list_keys;
pub use list_principals::list_principals;
pub use list_projects::{list_projects, ProjectFilter};
pub use put_grant::{put_grant, ProjectGrant};
pub use revoke_key::revoke_key;
pub use revoke_principal::revoke_principal;

pub struct ProjectAccess {
    pub id: GatewayProjectId,
    pub name: String,
    pub is_default: bool,
    pub role: String,
    pub client_ref: Option<String>,
}

pub struct GrantRow {
    pub user_id: UserId,
    pub role: String,
    pub client_ref: Option<String>,
}

pub struct PrincipalRow {
    pub id: GatewayPrincipalId,
    pub name: String,
    pub scopes: Vec<String>,
    pub client_ref: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub struct KeyRow {
    pub id: GatewayProjectKeyId,
    pub principal_id: GatewayPrincipalId,
    pub key_prefix: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub struct CredentialScope {
    pub account_id: AccountId,
    pub project_id: GatewayProjectId,
    pub principal_id: GatewayPrincipalId,
    pub scopes: Vec<String>,
    pub client_ref: Option<String>,
}
