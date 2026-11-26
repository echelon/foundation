use std::{fs, io, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{ensure, Context};
use db_queries::gateway_requests::artifacts as db_artifacts;
use ids::GatewayRequestId;
use tokio::sync::{OnceCell, Semaphore};

use super::{
    local_files::{self, PENDING_SUFFIX},
    artifact_storage::{generate_key, name, valid_key, OBJECT_SUFFIX},
    readiness::Readiness,
};

const ORPHAN_GRACE: Duration = Duration::from_secs(120);
const SWEEP_INTERVAL: Duration = Duration::from_secs(5);
const SWEEP_BATCH: usize = 64;

pub(super) struct FileStore {
    root: PathBuf,
    directory: OnceCell<PathBuf>,
    io_slots: Arc<Semaphore>,
}

impl FileStore {
    pub fn load() -> anyhow::Result<Option<Self>> {
        let Some(root) = std::env::var_os("TOKENCRAFT_DEMO_ARTIFACT_DIR") else {
            return Ok(None);
        };
        let root = PathBuf::from(root);
        local_files::require_private_directory(&root)
            .context("open the configured artifact directory")?;
        Ok(Some(Self {
            root: root
                .canonicalize()
                .context("resolve the artifact directory")?,
            directory: OnceCell::new(),
            io_slots: Arc::new(Semaphore::new(4)),
        }))
    }

    async fn directory(&self, pool: &db::PgPool) -> anyhow::Result<&PathBuf> {
        self.directory
            .get_or_try_init(|| async {
                let identity = db_artifacts::namespace(pool).await?;
                let root = self.root.clone();
                tokio::task::spawn_blocking(move || {
                    let path = root.join(identity);
                    let mut builder = fs::DirBuilder::new();
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::DirBuilderExt;
                        builder.mode(0o700);
                    }
                    match builder.create(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(error.into()),
                    }
                    local_files::require_private_directory(&path)?;
                    local_files::sync_directory(&root)?;
                    Ok(path)
                })
                .await?
            })
            .await
    }

    pub async fn write(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        payload: Vec<u8>,
    ) -> anyhow::Result<String> {
        let directory = self.directory(pool).await?.clone();
        let slot = self.io_slots.clone().acquire_owned().await?;
        let key = generate_key()?;
        let name = name(request, &key);
        tokio::task::spawn_blocking(move || {
            let _slot = slot;
            local_files::publish(&directory, &name, &payload)
        })
        .await??;
        Ok(key)
    }

    pub async fn read(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        key: &str,
        size: usize,
    ) -> anyhow::Result<Vec<u8>> {
        ensure!(valid_key(key), "invalid artifact key");
        let path = self.directory(pool).await?.join(name(request, key));
        let slot = self.io_slots.clone().acquire_owned().await?;
        tokio::task::spawn_blocking(move || {
            let _slot = slot;
            let payload = local_files::read_bounded(&path, size)?;
            ensure!(payload.len() == size, "artifact length mismatch");
            Ok(payload)
        })
        .await?
    }

    pub(super) async fn probe(&self, pool: &db::PgPool) -> anyhow::Result<()> {
        // Exercise the same storage path as a result, without creating a request or artifact record.
        const MARKER: &[u8] = b"TokenCraft artifact readiness";
        let directory = self.directory(pool).await?.clone();
        let path = directory.clone();
        tokio::task::spawn_blocking(move || local_files::require_private_directory(&path))
            .await??;
        let request = GatewayRequestId::generate();
        let key = self.write(pool, &request, MARKER.to_vec()).await?;
        let bytes = self.read(pool, &request, &key, MARKER.len()).await;
        tokio::fs::remove_file(directory.join(name(&request, &key))).await?;
        ensure!(
            bytes? == MARKER,
            "artifact readiness contents did not match"
        );
        Ok(())
    }

    pub(super) async fn sweep_forever(&self, pool: &db::PgPool, readiness: &Readiness) {
        let mut listing = None;
        let mut timer = tokio::time::interval(SWEEP_INTERVAL);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            readiness.work_tick(&mut timer).await;
            if let Err(error) = self.sweep(pool, &mut listing).await {
                tracing::warn!(error_kind = %error.root_cause(), "artifact file cleanup deferred");
                listing = None;
            }
        }
    }

    async fn sweep(
        &self,
        pool: &db::PgPool,
        listing: &mut Option<tokio::fs::ReadDir>,
    ) -> anyhow::Result<()> {
        let directory = self.directory(pool).await?;
        let entries = match listing {
            Some(entries) => entries,
            None => listing.insert(tokio::fs::read_dir(directory).await?),
        };
        let mut names = Vec::new();
        let mut requests = Vec::new();
        let mut keys = Vec::new();
        for _ in 0..SWEEP_BATCH {
            let Some(entry) = entries.next_entry().await? else {
                *listing = None;
                break;
            };
            let Some(filename) = entry.file_name().into_string().ok() else {
                continue;
            };
            let Some((request, key)) = identity(&filename) else {
                continue;
            };
            let Ok(metadata) = entry.metadata().await else {
                continue;
            };
            if !metadata.is_file()
                || !metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.elapsed().ok())
                    .is_some_and(|age| age >= ORPHAN_GRACE)
            {
                continue;
            }
            requests.push(request.to_owned());
            keys.push(key.to_owned());
            names.push(filename);
        }
        if names.is_empty() {
            return Ok(());
        }
        // The completion transaction publishes terminal state and file references together.
        // Taking both facts in one snapshot protects a writer or an unexpired download.
        for position in db_artifacts::disposable_files(pool, &requests, &keys).await? {
            let filename = &names[position as usize - 1];
            match tokio::fs::remove_file(directory.join(filename)).await {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    tracing::warn!(error_kind = ?error.kind(), "artifact file removal deferred");
                }
            }
        }
        Ok(())
    }
}

fn identity(filename: &str) -> Option<(&str, &str)> {
    let name = filename.strip_suffix(PENDING_SUFFIX).unwrap_or(filename);
    let (request, key) = name.strip_suffix(OBJECT_SUFFIX)?.split_once('-')?;
    (valid_key(key) && request.parse::<GatewayRequestId>().is_ok()).then_some((request, key))
}
