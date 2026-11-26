use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result, bail, ensure};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{ArtCraftReference, Catalog, Offer, Reference, Selection, digest, prepare_offer, read_file};

const HELP: &str = "Usage: api-gateway catalog check-reference --reference FILE --plugin-catalog FILE [--plugin-catalog FILE ...]

Validate public pricing fields and check that each ArtCraft preset is importable
and covered by an upstream model and capability set in a supplied plugin catalog.
The command reads local files and writes a JSON report with their exact hashes.
";
const MAX_SAFE_JS_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Deserialize)]
struct Publication {
    offers: Vec<PublicOffer>,
}

#[derive(Deserialize)]
struct PublicOffer {
    name: String,
    preset: String,
    source_url: String,
    rates: Vec<PublicRate>,
}

#[derive(Deserialize)]
struct PublicRate {
    unit: String,
    usd_nanos: u64,
    #[serde(default)]
    terms: String,
}

#[derive(Serialize)]
pub(super) struct Report<'a> {
    version: u8,
    reference_revision: &'a str,
    reference_sha256: String,
    offers: Vec<CheckedOffer<'a>>,
}

#[derive(Serialize)]
struct CheckedOffer<'a> {
    offer_id: &'a str,
    plugin: &'a str,
    upstream_model: &'a str,
    plugin_catalog_sha256: String,
    rate_count: usize,
}

pub(super) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let mut reference = None;
    let mut plugins = Vec::new();
    while let Some(option) = args.next() {
        match option.to_str() {
            Some("--reference") => {
                let path = args.next().context("set --reference FILE")?;
                ensure!(reference.replace(PathBuf::from(path)).is_none(), "supply one --reference FILE");
            }
            Some("--plugin-catalog") => plugins.push(PathBuf::from(
                args.next().context("set --plugin-catalog FILE")?,
            )),
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown reference check option. Use --help"),
        }
    }
    let reference = Reference::parse(read_file(&reference.context("set --reference FILE")?)?)?;
    let offers = project(&reference)?;
    let report = check(&reference, &offers, &plugins)?;
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &report)?;
    writeln!(out)?;
    Ok(())
}

pub(super) fn project(reference: &Reference) -> Result<Vec<Offer>> {
    let hash = digest(&reference.bytes);
    reference.catalog.offers.iter().map(|entry| {
        // Only the preset contract is under review here. Operators supply routing rates.
        let rates = entry.kind.pricing_fields().1.iter()
            .map(|&field| (field.into(), Value::from(0))).collect();
        let choice = Selection {
            reference: entry.id.clone(),
            binding: Some("publication-check".into()),
            id: None,
            model: None,
            update_from_sha256: None,
            rates: Some(rates),
        };
        let preset = ArtCraftReference {
            offer_id: entry.id.clone(),
            revision: reference.catalog.revision.clone(),
            sha256: hash.clone(),
            public_key_sha256: None,
        };
        let offer = prepare_offer(entry, choice, preset)
            .with_context(|| format!("prepare reference offer {}", entry.id))?;
        let bytes = serde_json::to_vec(&json!({ "version": 1, "offers": [offer] }))?;
        let mut catalog = Catalog::parse(&bytes)
            .with_context(|| format!("validate reference offer {}", entry.id))?;
        catalog.offers.pop().context("expected one projected offer")
    }).collect()
}

pub(super) fn check<'a>(
    reference: &'a Reference,
    offers: &'a [Offer],
    plugins: &[PathBuf],
) -> Result<Report<'a>> {
    ensure!(!plugins.is_empty(), "supply at least one --plugin-catalog FILE");
    let publication: Publication = serde_json::from_slice(&reference.bytes).context("read public pricing fields")?;
    let catalogs = plugins.iter().map(|path| {
        let bytes = read_file(path)?;
        let catalog = Catalog::parse(&bytes).with_context(|| format!("validate plugin catalog {}", path.display()))?;
        Ok((digest(bytes), catalog))
    }).collect::<Result<Vec<_>>>()?;

    let mut checked = Vec::with_capacity(offers.len());
    for (offer, presentation) in offers.iter().zip(publication.offers) {
        presentation.validate().with_context(|| format!("review public pricing for {}", offer.id))?;
        let model = offer.upstream_model.as_deref().context("reference offer requires an upstream model")?;
        let plugin = offer.provider.as_str();
        let (sha256, _) = catalogs.iter().find(|(_, catalog)| catalog.offers.iter().any(|candidate| {
            candidate.provider == offer.provider && candidate.adapter == offer.adapter
                && candidate.upstream_model.as_deref() == Some(model)
                && super::reference_adoption::covers(&candidate.pricing, &offer.pricing)
        })).with_context(|| format!("no {plugin} plugin catalog covers {} ({model})", offer.id))?;
        checked.push(CheckedOffer {
            offer_id: &offer.id, plugin, upstream_model: model,
            plugin_catalog_sha256: sha256.clone(), rate_count: presentation.rates.len(),
        });
    }
    Ok(Report {
        version: 1, reference_revision: &reference.catalog.revision,
        reference_sha256: digest(&reference.bytes), offers: checked,
    })
}

impl PublicOffer {
    fn validate(&self) -> Result<()> {
        ensure!(display_text(&self.name) && display_text(&self.preset), "name and preset must be readable text");
        let url = Url::parse(&self.source_url).map_err(|_| anyhow::anyhow!("invalid rate source URL"))?;
        ensure!(url.scheme() == "https" && url.host().is_some() && url.username().is_empty() && url.password().is_none(),
            "rate sources must use HTTPS without embedded credentials");
        ensure!(self.rates.iter().all(|rate| rate.usd_nanos <= MAX_SAFE_JS_INTEGER
                && display_text(&rate.unit) && (rate.terms.is_empty() || display_text(&rate.terms))),
            "list prices need a billing unit, an exact non-negative nanodollar amount, and readable terms when present");
        Ok(())
    }
}

fn display_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}
