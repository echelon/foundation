use std::{collections::BTreeMap, fs, io::Read, path::Path};

use anyhow::{ensure, Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::{copy_entry, local_port, new_file, private_entry, require_stopped, Stage};
use crate::{
    postgres::{Mode, Postgres, DATABASE},
    state::{private_dir, write_private, State},
};

const ROWS: &str = "removals.csv";
const SIGNATURE: &str = "signature";
const APPLIED: &str = "applied-requests";
const QUERY: &str = "
COPY (
    SELECT r.id, r.account_id, r.project_id, r.created_at, r.content_removed_at,
           r.succeeded_at, r.failed_at, r.canceled_at, e.id
    FROM gateway_requests r
    LEFT JOIN gateway_export_events e ON e.request_id = r.id
        AND e.event_type = 'request.content_removed' AND e.observation_key = ''
    WHERE r.content_removed_at IS NOT NULL
    ORDER BY r.id
) TO STDOUT CSV";
const CONTEXT: &[u8] = b"TokenCraft native content removal checkpoint v1";
const RECOVERY_SQL: &[u8] = include_bytes!("apply_removals.sql");

pub(super) async fn capture(source: &Path, destination: &Path) -> Result<()> {
    let state = State::open_existing(source)?;
    let stage = Stage::new(destination)?;
    let (postgres, _) = Postgres::start(&state, local_port()?, Mode::Backup).await?;
    let rows = stage.path.join(ROWS);
    let status = postgres.database_tool("psql", &state)?
        .stdout(new_file(&rows)?)
        .args(["-Xqw", "-v", "ON_ERROR_STOP=1", "-d", DATABASE, "-c", QUERY])
        .status().await.context("capture content removal checkpoint")?;
    ensure!(status.success(), "could not capture content removals");
    drop(postgres);
    require_stopped(&state)?;

    fs::File::open(&rows)?.sync_all()?;
    let code = hex::encode(checkpoint_mac(&rows, &state)?.finalize().into_bytes());
    write_private(&stage.path.join(SIGNATURE), code.as_bytes())?;
    stage.publish(destination)
}

pub(super) async fn apply(
    source: &Path,
    state: &State,
    postgres: &Postgres,
    url: &str,
) -> Result<()> {
    ensure!(private_entry(source)?.is_dir(), "choose a content removal checkpoint directory");
    let stage = state.root.join(".content-removals");
    private_dir(&stage)?;
    let mut copied = BTreeMap::new();
    for name in [ROWS, SIGNATURE] {
        ensure!(private_entry(&source.join(name))?.is_file(), "checkpoint entries must be files");
        copy_entry(source, &stage, Path::new(name), &mut copied)?;
    }
    let mut signature = Vec::new();
    fs::File::open(stage.join(SIGNATURE))?.take(65).read_to_end(&mut signature)?;
    let signature = hex::decode(signature).context("read checkpoint signature")?;
    checkpoint_mac(&stage.join(ROWS), state)?
        .verify_slice(&signature)
        .context("content removal checkpoint does not match this backup or was changed")?;

    // Upgrade only the private restoration database before applying recovered records.
    let pool = sqlx::PgPool::connect(url).await.context("connect to private recovery database")?;
    db::migrate(&pool).await.context("prepare recovery schema")?;
    pool.close().await;
    write_private(&stage.join("apply.sql"), RECOVERY_SQL)?;
    let status = postgres.database_tool("psql", state)?
        .current_dir(&stage)
        .args(["-Xqw", "-v", "ON_ERROR_STOP=1", "--single-transaction", "-d", DATABASE, "-f", "apply.sql"])
        .status().await.context("apply content removals")?;
    ensure!(status.success(), "could not apply content removal checkpoint");

    let ids = fs::read_to_string(stage.join(APPLIED))?;
    let ids: std::collections::BTreeSet<_> = ids.lines().collect();
    remove_artifact_files(&state.root.join("artifacts"), &ids)?;
    fs::remove_dir_all(stage)?;
    Ok(())
}

fn checkpoint_mac(rows: &Path, state: &State) -> Result<Hmac<Sha256>> {
    let key = hex::decode(&state.secrets.payload)?;
    let mut derivation = Hmac::<Sha256>::new_from_slice(&key)?;
    derivation.update(CONTEXT);
    let mut mac = Hmac::<Sha256>::new_from_slice(&derivation.finalize().into_bytes())?;
    let mut file = fs::File::open(rows)?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            return Ok(mac);
        }
        mac.update(&buffer[..size]);
    }
}

fn remove_artifact_files(root: &Path, requests: &std::collections::BTreeSet<&str>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let directory = entry?.path();
        if !private_entry(&directory)?.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let filename = entry.file_name();
            let Some(filename) = filename.to_str() else { continue };
            let filename = filename.strip_suffix(".pending").unwrap_or(filename);
            let Some((request, key)) = filename.strip_suffix(".tcm").and_then(|stem| stem.split_once('-'))
                else { continue };
            if requests.contains(request) && key.len() == 32 && key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                fs::remove_file(entry.path())?;
            }
        }
        fs::File::open(directory)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticates_the_checkpoint_bytes_for_one_state_lineage() -> Result<()> {
        let target = std::env::temp_dir().join("tokencraft-checkpoint-tests");
        let first = Stage::new(&target)?;
        let second = Stage::new(&target)?;
        let state = State::open(&first.path)?;
        let unrelated = State::open(&second.path)?;
        let rows = first.path.join(ROWS);
        write_private(&rows, b"fixture")?;
        let signature = checkpoint_mac(&rows, &state)?.finalize().into_bytes();
        assert!(checkpoint_mac(&rows, &state)?.verify_slice(&signature).is_ok());
        assert!(checkpoint_mac(&rows, &unrelated)?.verify_slice(&signature).is_err());
        write_private(&rows, b"changed")?;
        assert!(checkpoint_mac(&rows, &state)?.verify_slice(&signature).is_err());
        Ok(())
    }
}
