use std::{collections::BTreeMap, ffi::OsString, io, path::Path};

use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{apply_feed, load_reviewed_catalog, next_live_deadline, read_feed, Catalog};

const HELP: &str = "Usage: api-gateway catalog check-rate-feed --catalog FILE [--base-revision ID] [--feed FILE [--previous-feed FILE]]

Print the base revision for the catalog and configured bindings. Supply a known --base-revision to use it instead.
Pass --feed to check a proposal, optionally against the last accepted --previous-feed.
";

#[derive(Serialize)]
struct Report<'a> {
    base_revision: &'a str,
    catalog_file_revision: &'a str,
    rate_feed_offers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_valid_until: Option<BTreeMap<String, DateTime<Utc>>>,
}

pub(in crate::gateway_demo) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut catalog, mut revision, mut feed, mut previous) = (None, None, None, None);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--catalog") => &mut catalog,
            Some("--base-revision") => &mut revision,
            Some("--feed") => &mut feed,
            Some("--previous-feed") => &mut previous,
            Some("-h" | "--help") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown rate feed option. Use --help"),
        };
        let value = args.next().context("rate feed option needs a value")?;
        ensure!(slot.replace(value).is_none(), "duplicate rate feed option");
    }
    let catalog = load_reviewed_catalog(Path::new(&catalog.context("set --catalog FILE")?), revision)?;
    ensure!(
        previous.is_none() || feed.is_some(),
        "--previous-feed requires --feed"
    );
    let mut report = Report {
        base_revision: &catalog.revision,
        catalog_file_revision: &catalog.file_revision,
        rate_feed_offers: catalog.rate_feed_offers.clone(),
        revision: None,
        valid_until: None,
        rate_valid_until: None,
    };
    if let Some(feed) = feed {
        let (accepted, deadline) = check(
            &catalog,
            Path::new(&feed),
            previous.as_deref().map(Path::new),
        )?;
        report.rate_feed_offers = Some(accepted.rates.keys().cloned().collect());
        report.revision = Some(accepted.revision);
        report.valid_until = Some(deadline);
        report.rate_valid_until = Some(accepted.rates.into_iter()
            .map(|(offer, rate)| (offer, rate.valid_until)).collect());
    }
    serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

fn check(
    catalog: &Catalog,
    feed: &Path,
    previous: Option<&Path>,
) -> Result<(Catalog, DateTime<Utc>)> {
    let now = Utc::now();
    let previous = previous
        .map(|path| apply_feed(catalog, catalog, &read_feed(path)?, now))
        .transpose()
        .context("validate previous rate feed")?;
    let feed = read_feed(feed)?;
    let accepted = apply_feed(catalog, previous.as_ref().unwrap_or(catalog), &feed, now)?;
    let deadline = next_live_deadline(&accepted, now).context("proposed rate feed is already expired")?;
    Ok((accepted, deadline))
}
