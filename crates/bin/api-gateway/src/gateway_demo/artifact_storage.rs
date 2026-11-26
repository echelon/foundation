mod bucket;

use std::{sync::Arc, time::Duration};

use db_queries::gateway_requests::artifacts::ArtifactData;
use ids::GatewayRequestId;
use ring::rand::{SecureRandom, SystemRandom};

use super::{archive_blobs::{BAD_OBJECT, IO_ERROR}, artifact_files::FileStore, readiness::Readiness};
use bucket::BucketStore;

pub(super) struct ArtifactStorage {
    files: Option<Arc<FileStore>>,
    bucket: Option<Arc<BucketStore>>,
}

impl ArtifactStorage {
    pub(super) fn load() -> anyhow::Result<Option<Self>> {
        let files = FileStore::load()?.map(Arc::new);
        let bucket = BucketStore::load()?.map(Arc::new);
        Ok((files.is_some() || bucket.is_some()).then_some(Self { files, bucket }))
    }

    pub(super) fn backend(&self) -> &'static str {
        if self.bucket.is_some() {
            "s3"
        } else {
            "filesystem"
        }
    }

    pub(super) async fn write(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        payload: Vec<u8>,
    ) -> anyhow::Result<ArtifactData> {
        if let Some(bucket) = &self.bucket {
            return bucket.write(pool, request, payload).await.map(ArtifactData::Bucket);
        }
        let files = self.files.as_ref().ok_or_else(|| anyhow::anyhow!(IO_ERROR))?;
        files.write(pool, request, payload).await.map(ArtifactData::File)
    }

    pub(super) async fn read_file(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        key: &str,
        size: usize,
    ) -> Result<Vec<u8>, &'static str> {
        self.files.as_ref().ok_or(BAD_OBJECT)?
            .read(pool, request, key, size).await.map_err(|_| BAD_OBJECT)
    }

    pub(super) async fn read_bucket(
        &self,
        pool: &db::PgPool,
        request: &GatewayRequestId,
        key: &str,
        target: &str,
        size: usize,
    ) -> Result<Vec<u8>, &'static str> {
        self.bucket.as_ref().filter(|bucket| bucket.target() == target).ok_or(IO_ERROR)?
            .read(pool, request, key, size).await
    }

    pub(super) fn spawn(self: Arc<Self>, pool: db::PgPool, readiness: Arc<Readiness>, collect: bool) {
        if collect {
            if let Some(files) = &self.files {
                let (files, pool, readiness) = (Arc::clone(files), pool.clone(), Arc::clone(&readiness));
                tokio::spawn(async move { files.sweep_forever(&pool, &readiness).await });
            }
            if let Some(bucket) = &self.bucket {
                Arc::clone(bucket).spawn_cleanup(pool.clone(), Arc::clone(&readiness));
            }
        }
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(5));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                readiness.work_tick(&mut timer).await;
                let mut available = true;
                if let Some(files) = &self.files {
                    available = files.probe(&pool).await.is_ok();
                }
                if let Some(bucket) = &self.bucket {
                    available &= bucket.probe(&pool).await;
                }
                readiness.artifact_observed(available);
            }
        });
    }
}

pub(super) fn generate_key() -> anyhow::Result<String> {
    let mut key = [0u8; 16];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| anyhow::anyhow!("could not generate an artifact key"))?;
    Ok(hex::encode(key))
}

pub(super) const OBJECT_SUFFIX: &str = ".tcm";

pub(super) fn name(request: &GatewayRequestId, key: &str) -> String {
    format!("{request}-{key}{OBJECT_SUFFIX}")
}

pub(super) fn valid_key(key: &str) -> bool {
    key.len() == 32 && key.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
