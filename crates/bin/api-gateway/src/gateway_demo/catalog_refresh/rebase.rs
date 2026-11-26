use std::{ffi::OsString, path::PathBuf};

use anyhow::{bail, ensure, Context, Result};
use chrono::Utc;

use super::{apply_feed, next_live_deadline, read_feed, write_feed, Catalog};
use crate::gateway_demo::{
    catalog_import::review::require_preset_only, providers::pin_configured_catalog,
};

const HELP: &str = "Usage: api-gateway catalog rebase-rate-feed --base FILE --candidate FILE --feed FILE

Validate a current rate feed against both catalogs after a preset-only update.
Print the feed for the candidate with the original rates and validity windows.
";

pub(in crate::gateway_demo) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut base, mut candidate, mut feed) = (None, None, None);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--base") => &mut base,
            Some("--candidate") => &mut candidate,
            Some("--feed") => &mut feed,
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown rate feed rebase option. Use --help"),
        };
        let path = args.next().context("rate feed rebase option requires a file path")?;
        ensure!(slot.replace(PathBuf::from(path)).is_none(), "duplicate rate feed rebase option");
    }

    let mut base = Catalog::load(&base.context("set --base FILE")?).context("load the base catalog")?;
    let mut candidate = Catalog::load(&candidate.context("set --candidate FILE")?).context("load the candidate catalog")?;
    require_preset_only(&base, &candidate)?;
    pin_configured_catalog(&mut base, Some(&mut candidate))?;

    let mut feed = read_feed(&feed.context("set --feed FILE")?)?;
    let now = Utc::now();
    let accepted = apply_feed(&base, &base, &feed, now).context("validate the feed against the base catalog")?;
    ensure!(next_live_deadline(&accepted, now).is_some(), "rate feed is already expired");
    feed.base_revision = candidate.revision.clone();
    apply_feed(&candidate, &candidate, &feed, now).context("validate the feed against the candidate catalog")?;

    write_feed(&feed)
}
