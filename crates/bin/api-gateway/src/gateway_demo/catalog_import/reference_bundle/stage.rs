use std::{
    fs,
    io,
    path::Path,
};

use anyhow::{ensure, Context, Result};
use serde::Serialize;

use super::{verify_bundle, MAX_BUNDLE_BYTES};
use crate::gateway_demo::{catalog_import::reference_signature::VerifiedReference, local_files};

#[derive(Serialize)]
pub(super) struct StagedRelease<'a> {
    #[serde(flatten)]
    reference: &'a VerifiedReference,
    release_path: String,
    current_path: String,
}

pub(super) fn write<'a>(
    directory: &Path,
    reference: &'a VerifiedReference,
    public_key: &[u8],
    bundle: &[u8],
) -> Result<StagedRelease<'a>> {
    create_directory(directory)?;
    let release = directory.join(&reference.reference_sha256);
    let current = directory.join("current");
    create_directory(&release)?;
    create_directory(&current)?;
    local_files::sync_directory(directory)?;

    let filename = format!("{}.json", reference.public_key_sha256);
    let retained;
    let published = match publish_file(&release, &filename, bundle, false) {
        Ok(()) => bundle,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            retained = local_files::read_bounded(&release.join(&filename), MAX_BUNDLE_BYTES)
                .context("read the immutable reference bundle")?;
            ensure!(retained.len() <= MAX_BUNDLE_BYTES, "immutable reference bundle exceeds 96 KiB");
            let existing = verify_bundle(&retained, public_key)
                .context("verify the immutable reference bundle")?;
            ensure!(
                existing.verified.is_some_and(|release| release.reference_sha256 == reference.reference_sha256),
                "immutable release path contains a different reference snapshot"
            );
            &retained
        }
        Err(error) => return Err(error).context("write the immutable reference bundle"),
    };
    publish_file(&current, &filename, published, true).context("update the current reference bundle")?;
    Ok(StagedRelease {
        reference,
        release_path: format!("{}/{filename}", reference.reference_sha256),
        current_path: format!("current/{filename}"),
    })
}

fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).context("create the reference registry directory")?;
    ensure!(fs::symlink_metadata(path)?.is_dir(), "reference registry path must be a directory");
    Ok(())
}

fn publish_file(directory: &Path, name: &str, bytes: &[u8], replace: bool) -> io::Result<()> {
    let temporary_name = format!(".{name}-{:016x}", rand::random::<u64>());
    local_files::publish(directory, &temporary_name, bytes)?;
    let temporary = directory.join(temporary_name);
    let destination = directory.join(name);
    let result = if replace {
        fs::rename(&temporary, destination)
    } else {
        fs::hard_link(&temporary, destination)
    };
    let cleanup = fs::remove_file(temporary);
    result?;
    if !replace {
        cleanup?;
    }
    local_files::sync_directory(directory)
}
