use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::Context;

use super::super::{
    archive_blobs::{BAD_OBJECT, COLD_OBJECT, IO_ERROR},
    archive_bucket::ArchiveBucket,
    catalog::digest,
    local_files,
};

pub(super) enum Target {
    Directory { root: PathBuf, id: String },
    Bucket(Box<ArchiveBucket>),
}

impl Target {
    pub(super) fn directory(path: &Path) -> anyhow::Result<Self> {
        local_files::require_private_directory(path).context("open content archive directory")?;
        let root = path.canonicalize().context("resolve content archive directory")?;
        let id = digest(root.as_os_str().as_encoded_bytes());
        Ok(Self::Directory { root, id })
    }

    pub(super) fn bucket() -> anyhow::Result<Option<Self>> {
        Ok(ArchiveBucket::load(
            "TOKENCRAFT_CONTENT_ARCHIVE_S3",
            "tokencraft-content/v1",
            Some("TOKENCRAFT_CONTENT_ARCHIVE_S3_AUTO_RESTORE"),
        )?
        .map(|bucket| Self::Bucket(Box::new(bucket))))
    }

    pub(super) fn id(&self) -> &str {
        match self {
            Self::Directory { id, .. } => id,
            Self::Bucket(bucket) => bucket.target(),
        }
    }

    pub(super) fn permits_restore(&self) -> bool {
        matches!(self, Self::Bucket(bucket) if bucket.permits_restore())
    }

    pub(super) async fn request_restore(&self, name: &str) -> Result<(), &'static str> {
        match self {
            Self::Bucket(bucket) => bucket.request_restore(name).await,
            Self::Directory { .. } => Err(COLD_OBJECT),
        }
    }

    pub(super) async fn seal(&self, name: String, payload: Vec<u8>, limit: usize) -> Option<String> {
        let expected = digest(&payload);
        let verified = match self {
            Self::Directory { root, .. } => {
                let root = root.clone();
                tokio::task::spawn_blocking(move || {
                    let path = root.join(&name);
                    if !path.try_exists()? {
                        unlink(&root.join(format!("{name}{}", local_files::PENDING_SUFFIX)))?;
                        local_files::publish(&root, &name, &payload)?;
                    }
                    let bytes = local_files::read_bounded(&path, limit)?;
                    local_files::sync_directory(&root)?;
                    Ok::<_, io::Error>(bytes)
                })
                .await
                .ok()?
                .ok()?
            }
            Self::Bucket(bucket) => {
                // A process can stop after PUT and before its database commit. Verify the
                // deterministic object even when the conditional create reports a conflict.
                let _ = bucket.put(&name, payload).await;
                bucket.get(&name, limit).await.ok()?
            }
        };
        (digest(&verified) == expected).then_some(expected)
    }

    pub(super) async fn read(&self, name: &str, limit: usize) -> Result<Vec<u8>, &'static str> {
        match self {
            Self::Directory { root, .. } => {
                let path = root.join(name);
                tokio::task::spawn_blocking(move || local_files::read_bounded(&path, limit))
                    .await
                    .map_err(|_| IO_ERROR)?
                    .map_err(|error| match error.kind() {
                        io::ErrorKind::InvalidData => BAD_OBJECT,
                        _ => IO_ERROR,
                    })
            }
            Self::Bucket(bucket) => bucket.get(name, limit).await,
        }
    }

    pub(super) async fn purge(&self, names: Vec<String>) -> Result<(), &'static str> {
        match self {
            Self::Directory { root, .. } => {
                let root = root.clone();
                tokio::task::spawn_blocking(move || {
                    for name in names {
                        unlink(&root.join(&name))?;
                        unlink(&root.join(format!("{name}{}", local_files::PENDING_SUFFIX)))?;
                    }
                    local_files::sync_directory(&root)
                })
                .await
                .map_err(|_| IO_ERROR)?
                .map_err(|_| IO_ERROR)
            }
            Self::Bucket(bucket) => {
                for name in names {
                    bucket.delete(&name).await?;
                }
                Ok(())
            }
        }
    }
}

fn unlink(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
