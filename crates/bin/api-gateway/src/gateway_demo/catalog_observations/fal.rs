use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io,
};

use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    Client, Url,
};
use serde::{Deserialize, Serialize};

use super::{arguments, json_number_with_scale, pricing_client, read_response, read_snapshot};
use crate::gateway_demo::catalog::{digest, Adapter, Offer, Pricing};

const PRICING_URL: &str = "https://api.fal.ai/v1/models/pricing";
const MAX_ENDPOINTS_PER_REQUEST: usize = 50;
const HELP: &str = "Usage: api-gateway catalog observe-fal --catalog FILE [--input FILE]

Read fal endpoint prices using TOKENCRAFT_FAL_PRICING_KEY or parse a saved response.
Write the vendor billing units and operator pricing to stdout for review.
";

#[derive(Deserialize)]
struct PriceResponse {
    prices: Vec<FalPrice>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct FalPrice {
    endpoint_id: String,
    unit_price: serde_json::Number,
    unit: String,
    currency: String,
}

#[derive(Serialize)]
struct Report<'a> {
    version: u32,
    source_url: &'static str,
    source: &'static str,
    response_sha256s: Vec<String>,
    observed_at: Option<DateTime<Utc>>,
    catalog_file_revision: &'a str,
    offers: Vec<Observation<'a>>,
}

#[derive(Serialize)]
struct Observation<'a> {
    offer_id: &'a str,
    model: &'a str,
    upstream_model: &'a str,
    operator_pricing: &'a Pricing,
    supplier_pricing: Option<&'a FalPrice>,
    usd_nanos_per_vendor_unit: Option<i64>,
    status: &'static str,
}

pub(in crate::gateway_demo) async fn run(args: impl Iterator<Item = OsString>) -> Result<()> {
    let Some((catalog, input)) = arguments(args, HELP)? else {
        return Ok(());
    };
    let eligible: Vec<_> = catalog
        .offers
        .iter()
        .filter(|offer| {
            offer.provider.as_str() == "fal" && offer.adapter == Adapter::ConnectorQueue
        })
        .collect();
    ensure!(!eligible.is_empty(), "the catalog has no fal queue offers");
    let endpoints = eligible
        .iter()
        .map(|offer| {
            offer
                .upstream_model
                .as_deref()
                .context("fal offer is missing its endpoint ID")
        })
        .collect::<Result<BTreeSet<_>>>()?;

    let (source, responses, observed_at) = match input {
        Some(file) => ("file", vec![read_snapshot(&file)?], None),
        None => {
            let key = std::env::var("TOKENCRAFT_FAL_PRICING_KEY")
                .context("set TOKENCRAFT_FAL_PRICING_KEY or pass --input FILE")?;
            ensure!(
                !key.is_empty() && key.trim() == key,
                "invalid fal pricing key"
            );
            let mut authorization =
                HeaderValue::from_str(&format!("Key {key}")).context("invalid fal pricing key")?;
            authorization.set_sensitive(true);
            let endpoint = Url::parse(PRICING_URL)?;
            let responses =
                collect(&pricing_client()?, &endpoint, authorization, &endpoints).await?;
            ("authenticated_api", responses, Some(Utc::now()))
        }
    };
    let prices = index_prices(&responses)?;
    let offers = eligible
        .into_iter()
        .map(|offer| observe(offer, &prices))
        .collect::<Result<_>>()?;
    let report = Report {
        version: 1,
        source_url: PRICING_URL,
        source,
        response_sha256s: responses.iter().map(digest).collect(),
        observed_at,
        catalog_file_revision: &catalog.file_revision,
        offers,
    };
    serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

async fn collect(
    client: &Client,
    endpoint: &Url,
    authorization: HeaderValue,
    endpoints: &BTreeSet<&str>,
) -> Result<Vec<Vec<u8>>> {
    let parameters: Vec<_> = endpoints.iter().map(|id| ("endpoint_id", *id)).collect();
    let mut responses = Vec::new();
    for batch in parameters.chunks(MAX_ENDPOINTS_PER_REQUEST) {
        let response = client
            .get(endpoint.clone())
            .query(batch)
            .header(AUTHORIZATION, authorization.clone())
            .send()
            .await
            .context("read fal endpoint prices")?;
        responses.push(read_response(response).await?);
    }
    Ok(responses)
}

fn index_prices(responses: &[Vec<u8>]) -> Result<BTreeMap<String, FalPrice>> {
    let mut prices = BTreeMap::new();
    for bytes in responses {
        let snapshot: PriceResponse =
            serde_json::from_slice(bytes).context("decode fal pricing response")?;
        ensure!(
            !snapshot.has_more && snapshot.next_cursor.is_none(),
            "partial fal pricing response"
        );
        for price in snapshot.prices {
            ensure!(
                prices.insert(price.endpoint_id.clone(), price).is_none(),
                "duplicate fal endpoint ID"
            );
        }
    }
    Ok(prices)
}

fn observe<'a>(
    offer: &'a Offer,
    prices: &'a BTreeMap<String, FalPrice>,
) -> Result<Observation<'a>> {
    let upstream_model = offer
        .upstream_model
        .as_deref()
        .context("fal offer is missing its endpoint ID")?;
    let supplier_pricing = prices.get(upstream_model);
    let usd_nanos_per_vendor_unit = supplier_pricing
        .filter(|price| price.currency == "USD")
        .and_then(|price| vendor_unit_nanos(&price.unit_price));
    let status = match supplier_pricing {
        None => "endpoint_missing",
        Some(price) if price.currency != "USD" => "currency_review_required",
        Some(_) if usd_nanos_per_vendor_unit.is_none() => "amount_review_required",
        Some(_) => "review_required",
    };
    Ok(Observation {
        offer_id: &offer.id,
        model: &offer.model,
        upstream_model,
        operator_pricing: &offer.pricing,
        supplier_pricing,
        usd_nanos_per_vendor_unit,
        status,
    })
}

fn vendor_unit_nanos(price: &serde_json::Number) -> Option<i64> {
    json_number_with_scale(price, 9)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::OriginalUri, http::HeaderMap, routing::get, Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    #[test]
    fn parses_exact_numeric_rates_and_preserves_incompatible_units() {
        let responses = vec![br#"{"prices":[
            {"endpoint_id":"fal-ai/flux/dev","unit_price":2.5e-2,"unit":"megapixel","currency":"USD"}
        ],"has_more":false,"next_cursor":null}"#.to_vec()];
        let prices = index_prices(&responses).unwrap();
        assert_eq!(prices["fal-ai/flux/dev"].unit, "megapixel");
        assert_eq!(
            vendor_unit_nanos(&prices["fal-ai/flux/dev"].unit_price),
            Some(25_000_000)
        );
        assert_eq!(
            vendor_unit_nanos(&"1.5".parse().unwrap()),
            Some(1_500_000_000)
        );
        for value in ["-0.1", "1e-10", "1e+10"] {
            assert_eq!(vendor_unit_nanos(&value.parse().unwrap()), None, "{value}");
        }
    }

    #[tokio::test]
    async fn queries_only_requested_endpoints_in_bounded_authenticated_gets() {
        let api = Router::new().route(
            "/pricing",
            get(|uri: OriginalUri, headers: HeaderMap| async move {
                assert_eq!(headers[AUTHORIZATION], "Key fixture-pricing-key");
                let request = Url::parse(&format!("http://localhost{}", uri.0)).unwrap();
                let mut ids = Vec::new();
                for (name, value) in request.query_pairs() {
                    assert_eq!(name, "endpoint_id");
                    assert!(value.starts_with("fixture/model-"));
                    ids.push(value.into_owned());
                }
                assert!(matches!(ids.len(), 1 | MAX_ENDPOINTS_PER_REQUEST));
                let prices = ids.iter().map(|id| json!({
                "endpoint_id": id, "unit_price": 0.1, "unit": "video", "currency": "USD",
            })).collect::<Vec<_>>();
                Json(json!({"prices": prices, "has_more": false, "next_cursor": null}))
            }),
        );
        let bound = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Url::parse(&format!("http://{}/pricing", bound.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move { axum::serve(bound, api).await.unwrap() });
        let models = (1..=51)
            .map(|index| format!("fixture/model-{index}"))
            .collect::<Vec<_>>();
        let ids = models.iter().map(String::as_str).collect();
        let client = Client::builder().no_proxy().build().unwrap();
        let replies = collect(
            &client,
            &endpoint,
            HeaderValue::from_static("Key fixture-pricing-key"),
            &ids,
        )
        .await
        .unwrap();
        server.abort();
        assert_eq!(replies.len(), 2);
        assert_eq!(index_prices(&replies).unwrap().len(), 51);
    }
}
