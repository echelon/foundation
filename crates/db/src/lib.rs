//! Data layer: the connection pool and the embedded migrator. The ONLY SQL in the
//! codebase lives in `queries/` (not yet built out); migrations live in `migrations/`
//! and are applied via `admin-cli migrate`, never on service boot.

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

pub use sqlx::PgPool;

/// Migrations embedded at compile time from `crates/db/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a connection pool.
pub async fn connect(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Apply all pending migrations (used by `admin-cli migrate`).
pub async fn migrate(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Liveness check — `SELECT 1`.
pub async fn ping(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

/// Count application tables (excludes sqlx bookkeeping) — handy for `admin-cli db-info`.
pub async fn table_count(pool: &PgPool) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables \
         WHERE table_schema='public' AND table_type='BASE TABLE' AND table_name <> '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;
    Ok(n)
}

pub mod test_fixtures;
