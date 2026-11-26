use ids::{AccountId, GatewayMcpGrantId, GatewayProjectId, UserId};

mod create_grant;
mod exchange_code;
mod find_access;
mod list_grants;
mod revoke_grant;
mod revoke_token;
mod rotate_token;

pub use create_grant::{create_grant, NewGrant};
pub use exchange_code::{exchange_code, CodeExchange};
pub use find_access::find_access;
pub use list_grants::list_grants;
pub use revoke_grant::revoke_grant;
pub use revoke_token::{revoke_token, Revocation};
pub use rotate_token::rotate_token;

pub struct Grant {
    pub id: GatewayMcpGrantId,
    pub user_id: UserId,
    pub account_id: AccountId,
    pub project_id: GatewayProjectId,
    pub client_ref: Option<String>,
    pub oauth_client_id: String,
    pub scopes: Vec<String>,
}

pub struct Connection {
    pub id: GatewayMcpGrantId,
    pub project_id: GatewayProjectId,
    pub oauth_client_id: String,
    pub scopes: Vec<String>,
}

pub struct TokenRotation<'a> {
    pub previous_hash: &'a str,
    pub access_hash: &'a str,
    pub refresh_hash: &'a str,
    pub client: &'a str,
    pub resource: &'a str,
}
