use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

use anyhow::ensure;

pub(super) const PENDING_SUFFIX: &str = ".pending";

pub(super) fn require_private_directory(path: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(metadata.is_dir(), "storage path must be a directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "storage directory must have private permissions"
        );
    }
    Ok(())
}

pub(super) fn sync_directory(_directory: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(_directory)?.sync_all()?;
    Ok(())
}

pub(super) fn publish(directory: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let pending = directory.join(format!("{name}{PENDING_SUFFIX}"));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&pending)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::hard_link(&pending, directory.join(name))?;
        sync_directory(directory)
    })();
    drop(file);
    let _ = fs::remove_file(pending);
    result
}

pub(super) fn read_bounded(path: &Path, max: usize) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid object file",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((max + 1) as u64)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}
