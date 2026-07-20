//! Test-database harness (testing.md §3–§5).
//!
//! HARD RULE: tests and fixtures talk to `foundation_test` (or an ephemeral
//! `foundation_test_*`) ONLY. They read the connection string from exactly one env var —
//! `FOUNDATION_TEST_DATABASE_URL` — and [`guard_test_db`] PANICS before any connection if
//! that URL points at `foundation_development` or `foundation_production` (or any db whose
//! name doesn't mark it as a test db). This makes it impossible to wipe dev/prod from a test.

use crate::{connect, MIGRATOR, PgPool};

/// The one env var tests are allowed to read for the database (never `DATABASE_URL`).
pub const TEST_DB_ENV: &str = "FOUNDATION_TEST_DATABASE_URL";

/// Databases a test must NEVER touch.
const FORBIDDEN_DATABASES: &[&str] = &["foundation_development", "foundation_production"];

/// Connect to the test database (migrating it up first). Panics unless the URL is a test db.
pub async fn create_test_pool() -> PgPool {
    let url = std::env::var(TEST_DB_ENV).unwrap_or_else(|_| {
        panic!("{TEST_DB_ENV} must be set for DB tests (see .env.test) — tests never read DATABASE_URL")
    });
    guard_test_db(&url);
    let pool = connect(&url).await.expect("connect to test database");
    MIGRATOR.run(&pool).await.expect("migrate test database");
    pool
}

/// Panic unless `url` names a *_test database. The safety valve.
pub fn guard_test_db(url: &str) {
    let name = database_name(url).unwrap_or_default();

    assert!(
        !FORBIDDEN_DATABASES.contains(&name.as_str()),
        "REFUSING to run tests/fixtures against a non-test database: {name:?}. \
         Tests must use foundation_test (via {TEST_DB_ENV})."
    );
    assert!(
        name == "foundation_test" || name.starts_with("foundation_test_") || name.ends_with("_test"),
        "test database name must be foundation_test or foundation_test_* (got {name:?})"
    );
}

/// Extract the database name from a Postgres URL (last path segment, query stripped).
fn database_name(url: &str) -> Option<String> {
    let before_query = url.split('?').next()?;
    let name = before_query.rsplit('/').next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_rejects_development_and_production() {
        for db in ["foundation_development", "foundation_production"] {
            let url = format!("postgres://foundation:foundation@localhost/{db}");
            let err = std::panic::catch_unwind(|| guard_test_db(&url));
            assert!(err.is_err(), "guard must reject {db}");
        }
    }

    #[test]
    fn guard_accepts_test_databases() {
        for db in ["foundation_test", "foundation_test_abc123"] {
            let url = format!("postgres://foundation:foundation@localhost/{db}");
            guard_test_db(&url); // must not panic
        }
    }

    #[test]
    fn database_name_parses() {
        assert_eq!(
            database_name("postgres://u:p@localhost:5432/foundation_test?sslmode=disable").as_deref(),
            Some("foundation_test")
        );
    }
}
