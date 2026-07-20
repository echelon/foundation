//! admin-cli — migrations and ops one-offs (codebase_layout_backend.md §2). Migrations
//! are applied here (via `admin-cli migrate`), NEVER on service boot.

use anyhow::{bail, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Env files by convention (crates/bin/admin-cli/config/…) — no hand-supplied vars needed.
    let _ = config::load_env_files("admin-cli");
    let cfg = config::Config::load()?;
    let cmd = std::env::args().nth(1).unwrap_or_default();

    match cmd.as_str() {
        "migrate" => {
            let pool = db::connect(&cfg.database_url).await?;
            db::migrate(&pool).await?;
            let n = db::table_count(&pool).await?;
            println!("migrations applied · {n} application tables in {}", db_name(&cfg.database_url));
        }
        "db-info" => {
            let pool = db::connect(&cfg.database_url).await?;
            let up = db::ping(&pool).await;
            let n = db::table_count(&pool).await?;
            println!("database: {}", db_name(&cfg.database_url));
            println!("reachable: {up}");
            println!("application tables: {n}");
        }
        "" | "help" | "-h" | "--help" => {
            eprintln!("admin-cli <command>\n  migrate   apply pending migrations\n  db-info   show db name, reachability, table count");
        }
        other => bail!("unknown command: {other} (try `help`)"),
    }
    Ok(())
}

fn db_name(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|u| u.rsplit('/').next())
        .unwrap_or("?")
        .to_string()
}
