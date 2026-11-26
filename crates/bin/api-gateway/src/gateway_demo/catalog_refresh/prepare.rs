use std::{collections::BTreeMap, ffi::OsString, path::Path};

use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use serde::Deserialize;

use super::{apply_feed, deserialize_rates, digest, load_reviewed_catalog, next_live_deadline, write_feed, Rate, RateFeed};
use crate::gateway_demo::catalog_import::read_file;

const HELP: &str = "Usage: api-gateway catalog prepare-rate-feed --catalog FILE --approved FILE [--base-revision ID] [--previous-feed FILE]

Build a complete rate feed from explicitly approved offers and the last accepted feed.
Pin the catalog and binding revisions and, when updating, the previous file's SHA-256.
The validated candidate is written to stdout.
";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    version: u32,
    base_revision: String,
    catalog_file_revision: String,
    previous_feed_sha256: Option<String>,
    #[serde(deserialize_with = "deserialize_rates")]
    rates: BTreeMap<String, Rate>,
}

pub(in crate::gateway_demo) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut catalog, mut approval, mut revision, mut previous) = (None, None, None, None);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--catalog") => &mut catalog,
            Some("--approved") => &mut approval,
            Some("--base-revision") => &mut revision,
            Some("--previous-feed") => &mut previous,
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown rate feed preparation option. Use --help"),
        };
        let value = args.next().context("rate feed preparation option needs a value")?;
        ensure!(slot.replace(value).is_none(), "duplicate rate feed preparation option");
    }
    let catalog = load_reviewed_catalog(Path::new(&catalog.context("set --catalog FILE")?), revision)?;
    let bytes = read_file(Path::new(&approval.context("set --approved FILE")?))?;
    let approved: Approval = serde_json::from_slice(&bytes).context("decode approved rate changes")?;
    ensure!(approved.version == 1 && !approved.rates.is_empty(), "approval requires version 1 and at least one rate");
    ensure!(
        approved.base_revision == catalog.revision && approved.catalog_file_revision == catalog.file_revision,
        "approval does not match the reviewed catalog and bindings"
    );

    let now = Utc::now();
    let previous_bytes = previous.as_deref().map(Path::new).map(read_file).transpose()?;
    ensure!(
        approved.previous_feed_sha256 == previous_bytes.as_deref().map(digest),
        "approval previous_feed_sha256 does not match the previous feed"
    );
    let previous = previous_bytes.as_deref().map(serde_json::from_slice).transpose()
        .context("decode previous rate feed")?;
    let installed = previous.as_ref().map(|feed| apply_feed(&catalog, &catalog, feed, now))
        .transpose().context("validate previous rate feed")?;

    let mut candidate = previous.unwrap_or_else(|| RateFeed {
        base_revision: catalog.revision.clone(), observed_at: None, valid_until: None, rates: BTreeMap::new(),
    });
    // Preserve the original observation times when migrating a shared-window feed.
    if let (Some(observed), Some(until)) = (candidate.observed_at.take(), candidate.valid_until.take()) {
        for rate in candidate.rates.values_mut() {
            rate.observed_at = Some(observed);
            rate.valid_until = Some(until);
        }
    }
    candidate.rates.extend(approved.rates);
    let accepted = apply_feed(&catalog, installed.as_ref().unwrap_or(&catalog), &candidate, now)?;
    ensure!(next_live_deadline(&accepted, now).is_some(), "prepared rate feed is already expired");
    write_feed(&candidate)
}
