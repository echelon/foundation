use std::{
    collections::HashMap,
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
const REFERENCE_FIELDS: &[&str] = &["version", "revision", "publisher", "currency", "offers"];
const OFFER_FIELDS: &[&str] = &[
    "id", "model", "plugin", "upstream_model", "kind", "capabilities",
    "name", "preset", "source_url", "rates",
];

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
#[serde(deny_unknown_fields)]
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
    pub(super) offers: Vec<CheckedOffer<'a>>,
}

#[derive(Serialize)]
pub(super) struct CheckedOffer<'a> {
    offer_id: &'a str,
    plugin: String,
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
    let reference = Reference::load(&reference.context("set --reference FILE")?)?;
    let report = check(&reference, &plugins)?;
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
    plugins: &[PathBuf],
) -> Result<Report<'a>> {
    ensure!(!plugins.is_empty(), "supply at least one --plugin-catalog FILE");
    let rate_counts = validate_publication(reference)?;
    let offers = project(reference)?;
    let catalogs = plugins.iter().map(|path| {
        let bytes = read_file(path)?;
        let catalog = Catalog::parse(&bytes).with_context(|| format!("validate plugin catalog {}", path.display()))?;
        Ok((digest(bytes), catalog))
    }).collect::<Result<Vec<_>>>()?;
    let mut coverage = HashMap::new();
    for (sha256, catalog) in &catalogs {
        for offer in &catalog.offers {
            if let Some(model) = offer.upstream_model.as_deref() {
                coverage.entry((offer.provider.as_str(), offer.adapter.as_str(), model))
                    .or_insert_with(Vec::new)
                    .push((sha256, &offer.pricing));
            }
        }
    }

    let mut checked = Vec::with_capacity(offers.len());
    for ((entry, offer), rate_count) in reference.catalog.offers.iter().zip(offers).zip(rate_counts) {
        let model = entry.upstream_model.as_str();
        let plugin = offer.provider.as_str();
        let (sha256, _) = coverage.get(&(plugin, offer.adapter.as_str(), model))
            .and_then(|candidates| candidates.iter().find(|(_, pricing)| {
                super::reference_adoption::covers(pricing, &offer.pricing)
            }))
            .with_context(|| format!("no {plugin} plugin catalog covers {} ({model})", offer.id))?;
        checked.push(CheckedOffer {
            offer_id: &entry.id, plugin: plugin.to_owned(), upstream_model: model,
            plugin_catalog_sha256: (*sha256).clone(), rate_count,
        });
    }
    Ok(Report {
        version: 1, reference_revision: &reference.catalog.revision,
        reference_sha256: digest(&reference.bytes), offers: checked,
    })
}

pub(super) fn validate_publication(reference: &Reference) -> Result<Vec<usize>> {
    // Imports read only routing fields. Publication checks every field because it emits the original bytes.
    let snapshot: Value = serde_json::from_slice(&reference.bytes).context("read public catalog fields")?;
    let fields = snapshot.as_object().context("public catalog must be an object")?;
    ensure!(fields.keys().all(|key| REFERENCE_FIELDS.contains(&key.as_str())),
        "public catalog contains an unsupported top-level field");
    let offers = fields.get("offers").and_then(Value::as_array).context("read public catalog offers")?;
    for (index, offer) in offers.iter().enumerate() {
        let fields = offer.as_object().context("public offer must be an object")?;
        ensure!(fields.keys().all(|key| OFFER_FIELDS.contains(&key.as_str())),
            "public catalog offer {} contains an unsupported field", index + 1);
    }
    let publication: Publication = serde_json::from_slice(&reference.bytes).context("read public pricing fields")?;
    reference.catalog.offers.iter().zip(publication.offers).map(|(entry, presentation)| {
        presentation.validate().with_context(|| format!("review public pricing for {}", entry.id))?;
        Ok(presentation.rates.len())
    }).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_demo::catalog_import::tests::SNAPSHOT;

    #[test]
    fn publication_excludes_config_fields_at_each_level() {
        let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).unwrap();
        snapshot["offers"][0]["name"] = json!("Fixture Image");
        snapshot["offers"][0]["preset"] = json!("4:3 PNG");
        snapshot["offers"][0]["source_url"] = json!("https://example.invalid/pricing");
        let parse = |value: &Value| Reference::parse(serde_json::to_vec(value).unwrap()).unwrap();
        assert_eq!(validate_publication(&parse(&snapshot)).unwrap(), [1]);

        let mut invalid = snapshot.clone();
        invalid["connection_access"] = json!({});
        assert!(validate_publication(&parse(&invalid)).is_err());

        let mut invalid = snapshot.clone();
        invalid["offers"][0]["local_binding"] = json!("private-connector");
        assert!(validate_publication(&parse(&invalid)).is_err());

        snapshot["offers"][0]["rates"][0]["contract"] = json!("internal");
        assert!(validate_publication(&parse(&snapshot)).is_err());
    }
}
