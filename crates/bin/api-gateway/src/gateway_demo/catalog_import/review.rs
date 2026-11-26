use std::{collections::BTreeSet, ffi::OsString, io::{self, Write}, path::PathBuf};

use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

use super::{Catalog, CatalogFile};

const HELP: &str = "Usage: api-gateway catalog review --base FILE --candidate FILE [--require-preset-only]

Compare two validated catalogs. The JSON report uses local offer IDs in its paths.
Use --require-preset-only to allow changes to existing ArtCraft reference metadata.
";

#[derive(Serialize)]
struct Report {
    version: u8,
    base_catalog_file_revision: String,
    candidate_catalog_file_revision: String,
    preset_only: bool,
    changes: Vec<Change>,
}

#[derive(Serialize)]
struct Change {
    path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<Value>,
}

impl Change {
    fn is_preset_update(&self) -> bool {
        matches!(self.path.as_slice(), [offers, _, reference, field]
            if offers == "offers" && reference == "artcraft_reference"
                && matches!(field.as_str(), "revision" | "sha256" | "public_key_sha256"))
            && self.after.is_some()
    }
}

impl Report {
    fn require_preset_only(&self) -> Result<()> {
        if let Some(change) = self.changes.iter().find(|change| !change.is_preset_update()) {
            bail!("catalog review found a change outside the preset at {}. Run catalog review without --require-preset-only to see the report",
                change.path.join(" → "));
        }
        Ok(())
    }
}

pub(in crate::gateway_demo) fn require_preset_only(base: &Catalog, candidate: &Catalog) -> Result<()> {
    compare(base.clone(), candidate.clone())?.require_preset_only()
}

pub(super) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut base, mut candidate) = (None, None);
    let mut require_preset_only = false;
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--base") => &mut base,
            Some("--candidate") => &mut candidate,
            Some("--require-preset-only") => {
                ensure!(!require_preset_only, "duplicate catalog review option");
                require_preset_only = true;
                continue;
            }
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown catalog review option. Use --help"),
        };
        let path = args.next().context("catalog review option requires a file path")?;
        ensure!(slot.replace(PathBuf::from(path)).is_none(), "duplicate catalog review option");
    }

    let base = Catalog::load(&base.context("set --base FILE")?).context("load the base catalog")?;
    let candidate = Catalog::load(&candidate.context("set --candidate FILE")?).context("load the candidate catalog")?;
    let report = compare(base, candidate)?;
    if require_preset_only {
        report.require_preset_only()?;
    }
    let mut output = io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &report)?;
    writeln!(output)?;
    Ok(())
}

fn compare(base: Catalog, candidate: Catalog) -> Result<Report> {
    let base_catalog_file_revision = base.file_revision.clone();
    let candidate_catalog_file_revision = candidate.file_revision.clone();
    let before = document(base.into_file())?;
    let after = document(candidate.into_file())?;
    let mut changes = Vec::new();
    collect_changes(&mut Vec::new(), Some(&before), Some(&after), &mut changes);
    Ok(Report {
        version: 1,
        base_catalog_file_revision,
        candidate_catalog_file_revision,
        preset_only: changes.iter().all(Change::is_preset_update),
        changes,
    })
}

fn document(mut file: CatalogFile) -> Result<Value> {
    let mut offers = Map::new();
    for offer in std::mem::take(&mut file.offers) {
        let id = offer.id.clone();
        offers.insert(id, serde_json::to_value(offer)?);
    }
    let mut document = serde_json::to_value(file)?;
    document["offers"] = Value::Object(offers);
    Ok(document)
}

fn collect_changes(
    path: &mut Vec<String>,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<Change>,
) {
    if before == after {
        return;
    }
    if let (Some(Value::Object(left)), Some(Value::Object(right))) = (before, after) {
        for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
            path.push(key.clone());
            collect_changes(path, left.get(key), right.get(key), changes);
            path.pop();
        }
    } else {
        changes.push(Change { path: path.clone(), before: before.cloned(), after: after.cloned() });
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn catalog(value: &Value) -> Catalog {
        Catalog::parse(&serde_json::to_vec(value).unwrap()).unwrap()
    }

    #[test]
    fn compares_normalized_offers_by_id_and_preserves_exact_rate_changes() {
        let first = json!({
            "id": "first", "model": "sample", "provider": "fixture",
            "adapter": "mock", "mock_latency_ms": 100,
            "pricing": { "kind": "image", "usd_nanos_per_image": 30_000_001, "aspect_ratios": ["4:3", "1:1"] },
        });
        let mut other = first.clone();
        other["id"] = "other".into();
        let base = json!({"version": 1, "offers": [first, other]});
        let mut candidate = json!({"version": 1, "offers": [other, first]});
        candidate["offers"][1]["pricing"]["aspect_ratios"] = json!(["1:1", "8:6"]);
        let same = compare(catalog(&base), catalog(&candidate)).unwrap();
        assert_eq!(same.base_catalog_file_revision, same.candidate_catalog_file_revision);
        assert!(same.preset_only);
        assert!(same.changes.is_empty());

        candidate["offers"][1]["pricing"]["usd_nanos_per_image"] = 30_000_002.into();
        let report = compare(catalog(&base), catalog(&candidate)).unwrap();
        assert!(!report.preset_only);
        assert_eq!(serde_json::to_value(report.changes).unwrap(), json!([{
            "path": ["offers", "first", "pricing", "usd_nanos_per_image"],
            "before": 30_000_001, "after": 30_000_002,
        }]));
    }
}
