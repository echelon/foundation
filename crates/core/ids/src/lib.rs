//! Prefixed ULID string IDs. `{prefix}_{26-char lowercase Crockford ULID}` (ids.md).
//!
//! The prefix registry is the single source of truth for entity → prefix. In the full
//! design each entity gets a typed `{Entity}Id` newtype with compile-time prefix
//! uniqueness; here we keep a lean generator + the registry, enough to mint correct ids.

/// The prefix registry — one entry per table (schema.md §7). Compile-time uniqueness
/// is checked by [`assert_unique_prefixes`] (a test), mirroring the design's ids.md rule.
pub mod prefix {
    pub const ACCOUNT: &str = "acct";
    pub const USER: &str = "user";
    pub const SSO_CONNECTION: &str = "sso";
    pub const ACCOUNT_MEMBER: &str = "mem";
    pub const INVITATION: &str = "inv";
    pub const SESSION: &str = "sess";
    pub const API_KEY: &str = "key";
    pub const END_USER: &str = "enduser";
    pub const JOB: &str = "job";
    pub const JOB_EVENT: &str = "jevt";
    pub const FILE: &str = "file";
    pub const OUTPUT: &str = "out";
    pub const ASSET_INGEST: &str = "ingest";
    pub const QUEUE_TASK: &str = "task";
    pub const LEDGER_ENTRY: &str = "txn";
    pub const INVOICE: &str = "invoice";
    pub const BILLING_EVENT: &str = "bevt";
    pub const REQUEST_LOG: &str = "rlog";
    pub const PROVIDER_CALL: &str = "pcall";
    pub const WEBHOOK_DELIVERY: &str = "whd";
    pub const EMAIL_SEND: &str = "mail";
    pub const WEBHOOK_SECRET: &str = "whs";
    pub const IDEMPOTENCY_KEY: &str = "idem";
    pub const USAGE_ROLLUP: &str = "roll";
    pub const MODEL_USAGE_SUMMARY: &str = "sum";
    pub const CONTENT_LABEL: &str = "cl";
}

/// Mint a new prefixed id, e.g. `new_id(prefix::JOB)` → `job_01jabc…` (lowercase Crockford).
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", ulid::Ulid::new().to_string().to_lowercase())
}

/// The prefix of an id, if well-formed (`job_…` → `Some("job")`).
pub fn prefix_of(id: &str) -> Option<&str> {
    id.split_once('_').map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[&str] = &[
        prefix::ACCOUNT, prefix::USER, prefix::SSO_CONNECTION, prefix::ACCOUNT_MEMBER,
        prefix::INVITATION, prefix::SESSION, prefix::API_KEY, prefix::END_USER, prefix::JOB,
        prefix::JOB_EVENT, prefix::FILE, prefix::OUTPUT, prefix::ASSET_INGEST, prefix::QUEUE_TASK,
        prefix::LEDGER_ENTRY, prefix::INVOICE, prefix::BILLING_EVENT, prefix::REQUEST_LOG,
        prefix::PROVIDER_CALL, prefix::WEBHOOK_DELIVERY, prefix::EMAIL_SEND, prefix::WEBHOOK_SECRET,
        prefix::IDEMPOTENCY_KEY, prefix::USAGE_ROLLUP, prefix::MODEL_USAGE_SUMMARY, prefix::CONTENT_LABEL,
    ];

    #[test]
    fn assert_unique_prefixes() {
        let mut seen = std::collections::HashSet::new();
        for p in ALL {
            assert!(seen.insert(*p), "duplicate id prefix: {p}");
        }
    }

    #[test]
    fn shape_is_prefix_underscore_ulid() {
        let id = new_id(prefix::JOB);
        assert_eq!(prefix_of(&id), Some("job"));
        let (_, ulid) = id.split_once('_').unwrap();
        assert_eq!(ulid.len(), 26);
        assert_eq!(ulid, ulid.to_lowercase());
    }
}
