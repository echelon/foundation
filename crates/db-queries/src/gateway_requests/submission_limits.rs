use chrono::{DateTime, Utc};
use ids::{AccountId, UserId};
use sqlx::{PgConnection, PgPool};

#[derive(sqlx::FromRow)]
pub struct Limit {
    pub requests_per_minute: i32,
    pub burst: i32,
    pub next_self_service_limit: Option<i32>,
    pub next_self_service_at: Option<DateTime<Utc>>,
    pub operator_managed: bool,
}

pub async fn get(pool: &PgPool, account: &AccountId) -> sqlx::Result<Limit> {
    sqlx::query_as("SELECT * FROM gateway_submission_limit($1)")
        .bind(account.as_str())
        .fetch_one(pool)
        .await
}

pub async fn consume(conn: &mut PgConnection, account: &AccountId) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT gateway_consume_submission_credit($1)")
        .bind(account.as_str())
        .fetch_one(conn)
        .await
}

pub async fn increase(
    pool: &PgPool,
    account: &AccountId,
    user: &UserId,
    expected: i32,
    requested: i32,
) -> sqlx::Result<String> {
    sqlx::query_scalar("SELECT gateway_increase_submission_limit($1, $2, $3, $4)")
        .bind(account.as_str())
        .bind(user.as_str())
        .bind(expected)
        .bind(requested)
        .fetch_one(pool)
        .await
}
