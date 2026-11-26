pub(super) mod fal;
pub(super) mod kinovi;

use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::catalog::{digest, Adapter, Catalog, Offer, Pricing, MAX_RATE};

const MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const HELP: &str = "Usage: api-gateway catalog observe-openrouter --catalog FILE [--input FILE]

Read OpenRouter's public models API or parse a saved response from --input.
Write current catalog rates, observed list prices, and review status to stdout.
";

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Deserialize)]
struct OpenRouterModel {
    id: String,
    pricing: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Copy, Serialize)]
struct TokenRates {
    usd_nanos_per_1000_input_tokens: i64,
    usd_nanos_per_1000_output_tokens: i64,
}

#[derive(Serialize)]
struct Observation<'a> {
    offer_id: &'a str,
    model: &'a str,
    upstream_model: &'a str,
    operator_rates: TokenRates,
    supplier_pricing: Option<&'a BTreeMap<String, Value>>,
    list_rates: Option<TokenRates>,
    additional_price_fields: Vec<&'a str>,
    status: &'static str,
}

#[derive(Serialize)]
struct Report<'a> {
    version: u32,
    source_url: &'static str,
    source: &'static str,
    response_sha256: String,
    observed_at: Option<DateTime<Utc>>,
    catalog_file_revision: &'a str,
    offers: Vec<Observation<'a>>,
}

fn arguments(
    mut args: impl Iterator<Item = OsString>,
    help: &str,
) -> Result<Option<(Catalog, Option<PathBuf>)>> {
    let (mut catalog, mut input) = (None, None);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--catalog") => &mut catalog,
            Some("--input") => &mut input,
            Some("--help" | "-h") => {
                print!("{help}");
                return Ok(None);
            }
            _ => bail!("unknown observation option. Use --help"),
        };
        let path = args.next().context("observation option requires a file path")?;
        ensure!(slot.replace(PathBuf::from(path)).is_none(), "duplicate observation option");
    }
    let catalog = Catalog::load(&catalog.context("set --catalog FILE")?)?;
    Ok(Some((catalog, input)))
}

pub(super) async fn run(args: impl Iterator<Item = OsString>) -> Result<()> {
    let Some((catalog, input)) = arguments(args, HELP)? else {
        return Ok(());
    };
    let eligible: Vec<&Offer> = catalog.offers.iter()
        .filter(|offer| offer.provider.as_str() == "openrouter" && offer.adapter == Adapter::OpenAiCompatibleChat)
        .collect();
    ensure!(!eligible.is_empty(), "the catalog has no OpenRouter chat offers");

    let (source, bytes, observed_at) = match input {
        Some(file) => ("file", read_snapshot(&file)?, None),
        None => ("public_api", fetch_snapshot().await?, Some(Utc::now())),
    };
    let models: ModelsResponse = serde_json::from_slice(&bytes).context("decode OpenRouter models response")?;
    let mut index = HashMap::new();
    for model in &models.data {
        ensure!(index.insert(model.id.as_str(), model).is_none(), "duplicate OpenRouter model ID");
    }
    let offers = eligible.into_iter()
        .map(|offer| observation(offer, &index))
        .collect::<Result<_>>()?;
    let report = Report {
        version: 1,
        source_url: MODELS_URL,
        source,
        response_sha256: digest(&bytes),
        observed_at,
        catalog_file_revision: &catalog.file_revision,
        offers,
    };
    serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

fn read_snapshot(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path).with_context(|| format!("open {}", path.display()))?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_BYTES, "model response exceeds 8 MiB");
    Ok(bytes)
}

async fn fetch_snapshot() -> Result<Vec<u8>> {
    let response = pricing_client()?.get(MODELS_URL).send().await.context("read public OpenRouter models")?;
    read_response(response).await
}

fn pricing_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .no_proxy()
        .timeout(Duration::from_secs(12))
        .build()?)
}

async fn read_response(mut response: reqwest::Response) -> Result<Vec<u8>> {
    ensure!(response.status().is_success(), "supplier pricing endpoint replied with {}", response.status());
    ensure!(response.content_length().is_none_or(|size| size <= MAX_BYTES as u64), "model response exceeds 8 MiB");
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(chunk.len() <= MAX_BYTES - bytes.len(), "model response exceeds 8 MiB");
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn observation<'a>(offer: &'a Offer, models: &HashMap<&str, &'a OpenRouterModel>) -> Result<Observation<'a>> {
    let upstream_model = offer.upstream_model.as_deref().context("chat offer is missing its upstream model")?;
    let Pricing::Text { usd_nanos_per_1000_input_tokens, usd_nanos_per_1000_output_tokens } = offer.pricing else {
        bail!("OpenRouter observation requires a text offer");
    };
    let supplier = models.get(upstream_model);
    let supplier_pricing = supplier.and_then(|row| row.pricing.as_ref());
    let list_rates = supplier_pricing.and_then(token_prices);
    let additional_price_fields: Vec<_> = supplier_pricing.into_iter().flat_map(|pricing| pricing.iter())
        .filter(|(field, value)| {
            !matches!(field.as_str(), "prompt" | "completion")
                && value.as_str().and_then(nano_per_thousand) != Some(0)
        })
        .map(|(field, _)| field.as_str())
        .collect();
    let status = match (supplier, list_rates.is_some(), additional_price_fields.is_empty()) {
        (None, _, _) => "model_missing",
        (_, false, _) => "token_rates_unavailable",
        (_, true, false) => "review_required",
        (_, true, true) => "observed",
    };
    Ok(Observation {
        offer_id: &offer.id, model: &offer.model, upstream_model,
        operator_rates: TokenRates { usd_nanos_per_1000_input_tokens, usd_nanos_per_1000_output_tokens },
        supplier_pricing, list_rates, additional_price_fields, status,
    })
}

fn token_prices(pricing: &BTreeMap<String, Value>) -> Option<TokenRates> {
    Some(TokenRates {
        usd_nanos_per_1000_input_tokens: nano_per_thousand(pricing.get("prompt")?.as_str()?)?,
        usd_nanos_per_1000_output_tokens: nano_per_thousand(pricing.get("completion")?.as_str()?)?,
    })
}

fn nano_per_thousand(price: &str) -> Option<i64> {
    decimal_with_scale(price, 12).filter(|amount| *amount <= MAX_RATE)
}

fn json_number_with_scale(price: &serde_json::Number, places: i32) -> Option<i64> {
    let price = price.to_string();
    if let Some((mantissa, exponent)) = price.split_once(['e', 'E']) {
        decimal_with_scale(mantissa, places.checked_add(exponent.parse().ok()?)?)
    } else {
        decimal_with_scale(&price, places)
    }
}

fn decimal_with_scale(price: &str, places: i32) -> Option<i64> {
    let (whole, fraction) = price.split_once('.').unwrap_or((price, ""));
    if price.len() > 64 || whole.is_empty() || (price.contains('.') && fraction.is_empty())
        || !whole.bytes().chain(fraction.bytes()).all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let shift = places.checked_sub(fraction.len() as i32)?;
    let digits = format!("{whole}{fraction}");
    let significant = digits.trim_start_matches('0');
    if significant.is_empty() {
        return Some(0);
    }
    if shift < 0 {
        let cut = significant.len().checked_sub(shift.unsigned_abs() as usize)?;
        let (kept, remainder) = significant.split_at(cut);
        if remainder.bytes().any(|byte| byte != b'0') {
            return None;
        }
        kept.parse().ok()
    } else {
        significant.parse::<i64>().ok()?.checked_mul(10_i64.checked_pow(shift as u32)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_decimal_token_rates_with_exact_units() {
        assert_eq!(nano_per_thousand("0.0000004"), Some(400_000));
        assert_eq!(nano_per_thousand("0.0000000000010"), Some(1));
        assert_eq!(nano_per_thousand("0.001"), Some(MAX_RATE));
        for invalid in ["0.0000000000011", "-1", "1", "1e-6", ".", "0.", "..0", "+0"] {
            assert_eq!(nano_per_thousand(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn keeps_raw_charges_and_omits_an_unavailable_rate() {
        let catalog = Catalog::parse(br#"{"version":1,"offers":[{
            "id":"chat","provider":"openrouter","model":"chat",
            "adapter":"openai_compatible_chat","local_binding":"local",
            "upstream_model":"vendor/chat","pricing":{"kind":"text",
            "usd_nanos_per_1000_input_tokens":300,"usd_nanos_per_1000_output_tokens":500}
        }]}"#).unwrap();
        let model: OpenRouterModel = serde_json::from_str(r#"{
            "id":"vendor/chat","pricing":{"prompt":"0","completion":"-1","web_search":"0.01","request":"0"}
        }"#).unwrap();
        let result = observation(&catalog.offers[0], &HashMap::from([("vendor/chat", &model)])).unwrap();
        assert!(result.list_rates.is_none());
        assert_eq!(result.status, "token_rates_unavailable");
        assert_eq!(result.additional_price_fields, ["web_search"]);
        assert_eq!(result.operator_rates.usd_nanos_per_1000_input_tokens, 300);
    }
}
