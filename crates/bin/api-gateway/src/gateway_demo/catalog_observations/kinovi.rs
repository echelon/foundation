mod mcp;

use std::{ffi::OsString, io, path::Path};

use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{header::HeaderValue, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use super::{json_number_with_scale, pricing_client, read_snapshot};
use crate::gateway_demo::{
    catalog::{digest, Adapter, Catalog, Pricing},
    catalog_import::read_file,
};

const ESTIMATE_URL: &str = "https://kinovi.ai/api/mcp";
const HELP: &str = "Usage: api-gateway catalog observe-kinovi --catalog FILE --probe FILE [--input FILE] [--usd-nanos-per-credit INTEGER]

Estimate one configured Kinovi offer using TOKENCRAFT_KINOVI_PRICING_KEY or a saved estimate.
Write the credit estimate and any operator-specified USD conversion to stdout for review.
";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    version: u32,
    offer: String,
    inputs: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedEstimate {
    probe_sha256: String,
    estimate: Estimate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Estimate {
    model_id: String,
    credits: Number,
    original_credits: Option<Number>,
}

#[derive(Serialize)]
struct Report<'a> {
    version: u32,
    source: &'static str,
    source_url: &'static str,
    observed_at: Option<DateTime<Utc>>,
    response_sha256: String,
    probe_sha256: String,
    catalog_file_revision: &'a str,
    offer_id: &'a str,
    model: &'a str,
    upstream_model: &'a str,
    operator_pricing: &'a Pricing,
    estimated_credits: Number,
    original_credits: Option<Number>,
    usd_nanos_per_credit: Option<i64>,
    estimated_usd_nanos: Option<i64>,
    status: &'static str,
}

pub(in crate::gateway_demo) async fn run(mut args: impl Iterator<Item = OsString>) -> Result<()> {
    let (mut catalog, mut probe, mut input, mut conversion) = (None, None, None, None);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--catalog") => &mut catalog,
            Some("--probe") => &mut probe,
            Some("--input") => &mut input,
            Some("--usd-nanos-per-credit") => &mut conversion,
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            _ => bail!("unknown Kinovi observation option. Use --help"),
        };
        let value = args
            .next()
            .context("expected a value for Kinovi observation option")?;
        ensure!(
            slot.replace(value).is_none(),
            "duplicate Kinovi observation option"
        );
    }
    let conversion = conversion
        .map(|value| {
            value
                .to_str()
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|value| *value > 0)
                .context("--usd-nanos-per-credit must be a positive integer")
        })
        .transpose()?;
    let catalog = Catalog::load(Path::new(&catalog.context("set --catalog FILE")?))?;
    let probe_bytes = read_file(Path::new(&probe.context("set --probe FILE")?))?;
    let probe_sha256 = digest(&probe_bytes);
    let probe: Probe =
        serde_json::from_slice(&probe_bytes).context("decode Kinovi pricing probe")?;
    ensure!(
        probe.version == 1 && !probe.inputs.is_empty(),
        "expected a version 1 probe with inputs"
    );
    let offer = catalog
        .offers
        .iter()
        .find(|offer| offer.id == probe.offer)
        .filter(|offer| {
            offer.provider.as_str() == "kinovi" && offer.adapter == Adapter::ConnectorQueue
        })
        .context("probe must select an existing Kinovi queue offer")?;
    let upstream_model = offer
        .upstream_model
        .as_deref()
        .context("Kinovi offer is missing its upstream model")?;

    let (estimate, response_sha256, source, observed_at) = if let Some(path) = input {
        let snapshot = read_snapshot(Path::new(&path))?;
        let saved: SavedEstimate =
            serde_json::from_slice(&snapshot).context("decode saved Kinovi estimate")?;
        ensure!(
            saved.probe_sha256 == probe_sha256,
            "saved Kinovi estimate uses a different probe"
        );
        (saved.estimate, digest(&snapshot), "file", None)
    } else {
        let key = std::env::var("TOKENCRAFT_KINOVI_PRICING_KEY")
            .context("set TOKENCRAFT_KINOVI_PRICING_KEY or pass --input FILE")?;
        ensure!(
            !key.is_empty() && key.trim() == key,
            "invalid Kinovi pricing key"
        );
        let mut authorization = HeaderValue::from_str(&format!("Bearer {key}"))
            .context("invalid Kinovi pricing key")?;
        authorization.set_sensitive(true);
        let (estimate, response_sha256) = mcp::estimate(
            &pricing_client()?,
            &Url::parse(ESTIMATE_URL)?,
            authorization,
            upstream_model,
            &probe.inputs,
        )
        .await?;
        (
            estimate,
            response_sha256,
            "authenticated_api",
            Some(Utc::now()),
        )
    };
    ensure!(
        estimate.model_id == upstream_model,
        "Kinovi estimate returned a different model"
    );
    ensure!(
        !estimate.credits.as_str().starts_with('-')
            && estimate
                .original_credits
                .as_ref()
                .is_none_or(|credits| !credits.as_str().starts_with('-')),
        "Kinovi returned negative credits"
    );
    let estimated_usd_nanos = conversion.and_then(|rate| usd_cost(&estimate.credits, rate));
    let status = match (conversion, estimated_usd_nanos) {
        (None, _) => "conversion_required",
        (Some(_), None) => "amount_review_required",
        (Some(_), Some(_)) => "review_required",
    };
    let report = Report {
        version: 1,
        source,
        source_url: ESTIMATE_URL,
        observed_at,
        response_sha256,
        probe_sha256,
        catalog_file_revision: &catalog.file_revision,
        offer_id: &offer.id,
        model: &offer.model,
        upstream_model,
        operator_pricing: &offer.pricing,
        estimated_credits: estimate.credits,
        original_credits: estimate.original_credits,
        usd_nanos_per_credit: conversion,
        estimated_usd_nanos,
        status,
    };
    serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

fn usd_cost(credits: &Number, usd_nanos_per_credit: i64) -> Option<i64> {
    let microcredits = json_number_with_scale(credits, 6)?;
    let product = i128::from(microcredits) * i128::from(usd_nanos_per_credit);
    if product % 1_000_000 != 0 {
        return None;
    }
    (product / 1_000_000).try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_fractional_credits_exactly_and_rejects_rounding() {
        assert_eq!(
            usd_cost(&"2.17".parse().unwrap(), 4_710_000),
            Some(10_220_700)
        );
        assert_eq!(usd_cost(&"2e-1".parse().unwrap(), 100), Some(20));
        assert_eq!(usd_cost(&"0.5".parse().unwrap(), 1), None);
        assert_eq!(usd_cost(&"0.0000001".parse().unwrap(), 1), None);
        assert_eq!(usd_cost(&"2".parse().unwrap(), i64::MAX), None);
    }
}
