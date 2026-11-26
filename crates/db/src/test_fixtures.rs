//! PostgreSQL fixtures for tests. See `FOUNDATION_TEST_DATABASE_URL`.

use crate::{MIGRATOR, PgPool};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

/// The test database URL. The harness never reads `DATABASE_URL`.
pub const TEST_DB_ENV: &str = "FOUNDATION_TEST_DATABASE_URL";

const DEFAULT_TEST_DATABASE_URL: &str =
    "postgres://foundation:foundation@localhost:5432/foundation_test";

/// Connect to a test database and apply migrations before returning the pool.
pub async fn create_test_pool() -> PgPool {
    let url = std::env::var(TEST_DB_ENV).unwrap_or_else(|_| DEFAULT_TEST_DATABASE_URL.to_string());
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(test_db_options(&url))
        .await
        .expect("connect to test database");
    MIGRATOR.run(&pool).await.expect("migrate test database");
    pool
}

/// Panic unless the URL resolves to an explicitly named test database.
pub fn guard_test_db(url: &str) {
    test_db_options(url);
}

fn test_db_options(url: &str) -> PgConnectOptions {
    let options: PgConnectOptions = url.parse().unwrap_or_else(|_| {
        panic!("{TEST_DB_ENV} must contain a valid PostgreSQL URL");
    });
    let database = options.get_database().unwrap_or_default();
    assert!(
        database.starts_with("foundation_test_") || database.ends_with("_test"),
        "{TEST_DB_ENV} must select a foundation_test_* or *_test database (got {database:?})",
    );
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_test_database_names_and_url_options() {
        for database in [
            "foundation_test",
            "foundation_test_run_123",
            "tokencraft_test",
        ] {
            let url = format!("postgres://localhost/{database}?sslmode=disable");
            guard_test_db(&url);
        }
    }

    #[test]
    fn rejects_non_test_database_names() {
        for database in ["foundation_development", "foundation_production", "other"] {
            let url = format!("postgres://localhost/{database}");
            assert!(
                std::panic::catch_unwind(|| guard_test_db(&url)).is_err(),
                "allowed {database}",
            );
        }
    }

    #[test]
    #[should_panic(expected = "got \"foundation_production\"")]
    fn rejects_a_url_whose_explicit_database_overrides_the_path() {
        guard_test_db("postgres://localhost/foundation_test?dbname=foundation_production");
    }
}
