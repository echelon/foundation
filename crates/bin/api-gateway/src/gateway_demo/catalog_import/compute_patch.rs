use std::{ffi::OsString, io, path::PathBuf};

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};

use crate::gateway_demo::{catalog::valid_identifier, compute_providers::ComputePatch};
use super::read_file;

pub(super) fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut base, mut bundle) = (None, None);
    while let Some(arg) = args.next() {
        let target = match arg.to_str() {
            Some("--base") => &mut base,
            Some("--bundle") => &mut bundle,
            _ => bail!("usage: api-gateway catalog apply-compute-patch [--base FILE] --bundle FILE"),
        };
        let value = args.next().context("expected a file path")?;
        ensure!(target.replace(PathBuf::from(value)).is_none(), "duplicate option");
    }
    let patch: ComputePatch = serde_json::from_slice(&read_file(&bundle.context("set --bundle FILE")?)?)?;
    let base = base.as_deref().map(read_file).transpose()?
        .map(|bytes| serde_json::from_slice(&bytes)).transpose()?
        .unwrap_or_else(|| json!({"version": 1, "bindings": []}));
    let next = merge(base, patch)?;
    let bytes = serde_json::to_vec_pretty(&next)?;
    ensure!(bytes.len() <= 16 * 1024, "local chat bindings exceed 16 KiB");
    std::io::Write::write_all(&mut io::stdout().lock(), &bytes)?;
    Ok(())
}

fn merge(mut base: Value, mut patch: ComputePatch) -> Result<Value> {
    ensure!(
        patch.version == 1 && valid_identifier(&patch.cell) && valid_identifier(&patch.binding)
            && patch.target.organization_id.is_some(), "invalid compute patch scope"
    );
    patch.target.endpoint = patch.target.validate()?.to_string();
    ensure!(base["version"] == 1, "expected version 1 local chat bindings");
    if let Some(cell) = base.get("cell").filter(|value| !value.is_null()) {
        ensure!(cell.as_str() == Some(patch.cell.as_str()), "compute registry cell changed");
    }
    base["cell"] = json!(patch.cell);
    let bindings = base["bindings"].as_array_mut().context("expected local chat bindings")?;
    let location = serde_json::to_value(&patch.target.location)?;
    let target = serde_json::to_value(patch.target)?;
    let Some(binding) = bindings.iter_mut().find(|binding| binding["name"] == patch.binding) else {
        ensure!(bindings.len() < 16, "local chat already has 16 bindings");
        bindings.push(json!({
            "name": patch.binding, "allowed_locations": [location], "targets": [target],
        }));
        return Ok(base);
    };
    let targets = binding["targets"].as_array_mut().context("choose a compute pool or a new binding name")?;
    ensure!(targets.len() < 4, "this pool already has four targets");
    ensure!(!targets.iter().any(|item| item["id"] == target["id"] || item["endpoint"] == target["endpoint"]),
        "target ID or endpoint is already configured");
    targets.push(target);
    let locations = binding["allowed_locations"].as_array_mut().context("expected allowed compute locations")?;
    if !locations.contains(&location) {
        ensure!(locations.len() < 4, "this pool already declares four allowed locations");
        locations.push(location);
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ids::AccountId;

    fn patch() -> ComputePatch {
        serde_json::from_value(json!({
            "version": 1, "cell": "test", "binding": "chat", "target": {
                "id": "fallback", "location": {"cloud": "aws", "region": "test"},
                "zone": "regional", "priority": 20, "endpoint": "https://fixture.example/v1/chat/completions",
                "key_env": "TOKENCRAFT_LOCAL_CHAT_KEY_TEST", "organization_id": AccountId::generate()
            }
        })).unwrap()
    }

    #[test]
    fn adds_scoped_targets_without_replacing_existing_bindings() {
        let base = json!({"version": 1, "bindings":[{"name":"local", "endpoint":"http://127.0.0.1:1024/v1/chat/completions"}]});
        let next = merge(base.clone(), patch()).unwrap();
        assert_eq!(next["bindings"][0], base["bindings"][0]);
        assert_eq!(next["bindings"][1]["targets"][0]["id"], "fallback");
        assert!(next["bindings"][1]["targets"][0]["organization_id"].is_string());
        assert!(merge(next, patch()).is_err());
    }
}
