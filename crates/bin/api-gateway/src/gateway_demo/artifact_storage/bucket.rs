use std::{sync::Arc, time::Duration};

use anyhow::ensure;
use db_queries::gateway_requests::{artifact_objects, artifacts};
use ids::GatewayRequestId;
use tokio::{sync::{OnceCell, Semaphore}, task::JoinSet};

use super::{generate_key, name, valid_key};
use crate::gateway_demo::{
    archive_blobs::{BAD_OBJECT, IO_ERROR},
    archive_bucket::ArchiveBucket,
    catalog::digest,
    readiness::Readiness,
};

pub(super) struct BucketStore {
    bucket: ArchiveBucket,
    namespace: OnceCell<String>,
    io_slots: Semaphore,
}

impl BucketStore {
    pub(super) fn load() -> anyhow::Result<Option<Self>> {
        Ok(ArchiveBucket::load("TOKENCRAFT_ARTIFACT_S3", "tokencraft-artifacts/v1", None)?
            .map(|bucket| Self { bucket, namespace: OnceCell::new(), io_slots: Semaphore::new(4) }))
    }

    pub(super) fn target(&self) -> &str {
        self.bucket.target()
    }

    async fn namespace(&self, pool: &db::PgPool) -> sqlx::Result<&str> {
        let namespace = self.namespace.get_or_try_init(|| artifacts::namespace(pool)).await?;
        Ok(namespace)
    }

    pub(super) async fn write(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        payload: Vec<u8>,
    ) -> anyhow::Result<String> {
        let _slot = self.io_slots.acquire().await?;
        let key = generate_key()?;
        let name = format!("{}/{}", self.namespace(pool).await?, name(request, &key));
        artifact_objects::reserve(pool, request, &key, self.target()).await?;
        let size = payload.len();
        let checksum = digest(&payload);
        // The upload may have succeeded even if the connection closed. Only publish
        // an artifact after the bucket returns the exact ciphertext for its key.
        let _ = self.bucket.put(&name, payload).await;
        let bytes = self.bucket.get(&name, size).await.map_err(|code| anyhow::anyhow!(code))?;
        ensure!(bytes.len() == size && digest(&bytes) == checksum, "artifact upload verification failed");
        Ok(key)
    }

    pub(super) async fn read(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        key: &str,
        size: usize,
    ) -> Result<Vec<u8>, &'static str> {
        if !valid_key(key) {
            return Err(BAD_OBJECT);
        }
        let _slot = self.io_slots.acquire().await.map_err(|_| IO_ERROR)?;
        let namespace = self.namespace(pool).await.map_err(|_| IO_ERROR)?;
        let bytes = self.bucket.get(&format!("{namespace}/{}", name(request, key)), size).await?;
        if bytes.len() != size {
            return Err(BAD_OBJECT);
        }
        Ok(bytes)
    }

    pub(super) async fn probe(&self, pool: &db::PgPool) -> bool {
        // Keep one small marker per database. Normal checks only read it.
        const MARKER: &[u8] = b"TokenCraft artifact storage v1";
        let Ok(namespace) = self.namespace(pool).await else { return false };
        let name = format!("{namespace}/readiness.tcm");
        match self.bucket.get(&name, MARKER.len()).await {
            Ok(body) => body == MARKER,
            Err(IO_ERROR) => {
                let _ = self.bucket.put(&name, MARKER.to_vec()).await;
                self.bucket.get(&name, MARKER.len()).await.is_ok_and(|body| body == MARKER)
            }
            Err(_) => false,
        }
    }

    pub(super) fn spawn_cleanup(self: Arc<Self>, pool: db::PgPool, readiness: Arc<Readiness>) {
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(2));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                readiness.work_tick(&mut timer).await;
                if let Err(error) = self.cleanup(&pool).await {
                    tracing::warn!(sqlstate = ?error.as_database_error().and_then(|e| e.code()),
                        "artifact bucket cleanup deferred");
                }
            }
        });
    }

    async fn cleanup(self: &Arc<Self>, pool: &db::PgPool) -> sqlx::Result<()> {
        let namespace = self.namespace(pool).await?;
        let mut deletions = JoinSet::new();
        for task in artifact_objects::claim_deletions(pool, self.target()).await? {
            let (Ok(request), true) = (task.request_id.parse(), valid_key(&task.object_key)) else {
                continue;
            };
            let name = format!("{namespace}/{}", name(&request, &task.object_key));
            let (store, pool) = (Arc::clone(self), pool.clone());
            deletions.spawn(async move {
                if store.bucket.delete(&name).await.is_ok() {
                    artifact_objects::deleted(&pool, &task).await?;
                } else {
                    tracing::warn!("artifact bucket object removal deferred");
                }
                Ok::<_, sqlx::Error>(())
            });
        }
        while let Some(result) = deletions.join_next().await {
            match result {
                Ok(Err(error)) => tracing::warn!(sqlstate = ?error.as_database_error().and_then(|e| e.code()),
                    "recording artifact bucket removal deferred"),
                Err(_) => tracing::warn!("artifact bucket cleanup task interrupted"),
                Ok(Ok(())) => {}
            }
        }
        Ok(())
    }
}
