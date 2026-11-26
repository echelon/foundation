pub(super) mod check;
pub(super) mod prepare;
pub(super) mod rebase;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{ensure, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::catalog::{digest, Catalog, Pricing, VerifiedRate, MAX_RATE};

const MAX_FEED_BYTES: u64 = 64 * 1024;

fn load_reviewed_catalog(path: &Path, revision: Option<OsString>) -> anyhow::Result<Catalog> {
    let mut catalog = Catalog::load(path)?;
    if let Some(revision) = revision {
        let revision = revision.into_string().map_err(|_| anyhow::anyhow!("invalid gateway base revision"))?;
        ensure!(
            revision.strip_prefix("demo_").is_some_and(|hash| {
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            "invalid gateway base revision"
        );
        catalog.revision = revision;
    } else {
        super::providers::pin_configured_catalog(&mut catalog, None)?;
    }
    Ok(catalog)
}

pub(super) struct CatalogStore {
    reviewed: Arc<Catalog>,
    current: RwLock<Arc<Catalog>>,
    feed_path: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RateFeed {
    base_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_until: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "deserialize_rates")]
    rates: BTreeMap<String, Rate>,
}

fn deserialize_rates<'de, D>(input: D) -> Result<BTreeMap<String, Rate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Rates;
    impl<'de> serde::de::Visitor<'de> for Rates {
        type Value = BTreeMap<String, Rate>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("rates keyed by distinct offer IDs")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut rates = BTreeMap::new();
            while let Some((id, rate)) = map.next_entry()? {
                if rates.insert(id, rate).is_some() {
                    return Err(serde::de::Error::custom("duplicate offer ID in rate feed"));
                }
            }
            Ok(rates)
        }
    }
    input.deserialize_map(Rates)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Rate {
    source_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_until: Option<DateTime<Utc>>,
    schedule: RateSchedule,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RateSchedule {
    Image {
        usd_nanos_per_image: i64,
    },
    Text {
        usd_nanos_per_1000_input_tokens: i64,
        usd_nanos_per_1000_output_tokens: i64,
    },
    Video {
        usd_nanos_per_second: i64,
    },
    Audio {
        usd_nanos_per_second: i64,
    },
    Mesh {
        usd_nanos_per_mesh: i64,
    },
}

impl CatalogStore {
    pub(super) fn load(reviewed: Arc<Catalog>) -> anyhow::Result<Self> {
        let feed_path = std::env::var_os("TOKENCRAFT_DEMO_RATE_FEED").map(PathBuf::from);
        ensure!(
            reviewed.rate_feed_offers.is_none() || feed_path.is_some(),
            "set TOKENCRAFT_DEMO_RATE_FEED for the configured rate_feed_offers"
        );
        let store = Self {
            current: RwLock::new(Arc::clone(&reviewed)),
            reviewed,
            feed_path,
        };
        store.refresh()?;
        Ok(store)
    }

    pub(super) fn reviewed(&self) -> &Catalog {
        &self.reviewed
    }

    pub(super) fn current(&self) -> Arc<Catalog> {
        Arc::clone(
            &self
                .current
                .read()
                .unwrap_or_else(|error| error.into_inner()),
        )
    }

    pub(super) fn spawn(self: &Arc<Self>) {
        if self.feed_path.is_none() {
            return;
        }
        let store = Arc::clone(self);
        tokio::spawn(async move {
            let mut failed = false;
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;
                let store = Arc::clone(&store);
                let result = tokio::task::spawn_blocking(move || store.refresh()).await;
                let next_failed = !matches!(result, Ok(Ok(())));
                if next_failed && !failed {
                    tracing::warn!("catalog rate feed rejected; keeping last verified revision");
                } else if !next_failed && failed {
                    tracing::info!("catalog rate feed recovered");
                }
                failed = next_failed;
            }
        });
    }

    fn refresh(&self) -> anyhow::Result<()> {
        let Some(path) = &self.feed_path else {
            return Ok(());
        };
        let metadata = std::fs::metadata(path).context("inspect catalog rate feed")?;
        ensure!(metadata.is_file(), "catalog rate feed must be a regular file");
        let feed = read_feed(path)?;
        let previous = self.current();
        let next = apply_feed(&self.reviewed, &previous, &feed, Utc::now())?;
        *self
            .current
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Arc::new(next);
        Ok(())
    }
}

fn read_feed(path: &Path) -> anyhow::Result<RateFeed> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .context("open catalog rate feed")?
        .take(MAX_FEED_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_FEED_BYTES,
        "catalog rate feed exceeds 64 KiB"
    );
    serde_json::from_slice(&bytes).context("decode catalog rate feed")
}

fn write_feed(feed: &RateFeed) -> anyhow::Result<()> {
    let mut output = serde_json::to_vec_pretty(feed)?;
    if output.len() as u64 >= MAX_FEED_BYTES {
        output = serde_json::to_vec(feed)?;
    }
    ensure!(output.len() as u64 <= MAX_FEED_BYTES, "rate feed exceeds 64 KiB");
    if (output.len() as u64) < MAX_FEED_BYTES {
        output.push(b'\n');
    }
    io::stdout().lock().write_all(&output)?;
    Ok(())
}

fn apply_feed(
    base: &Catalog,
    previous: &Catalog,
    feed: &RateFeed,
    now: DateTime<Utc>,
) -> anyhow::Result<Catalog> {
    ensure!(
        feed.base_revision == base.revision,
        "rate feed does not match the reviewed catalog"
    );
    ensure!(
        !feed.rates.is_empty() && feed.rates.len() <= base.offers.len(),
        "invalid rate feed size"
    );
    if let Some(ids) = &base.rate_feed_offers {
        ensure!(feed.rates.keys().eq(ids), "rate feed must cover exactly the configured rate_feed_offers");
    } else {
        ensure!(previous.rates.is_empty() || previous.rates.keys().eq(feed.rates.keys()), "incomplete rate feed");
    }
    let mut next = base.clone();
    for (id, rate) in &feed.rates {
        let (observed_at, valid_until) = match (feed.observed_at, feed.valid_until, rate.observed_at, rate.valid_until) {
            (Some(observed), Some(until), None, None) | (None, None, Some(observed), Some(until)) => (observed, until),
            _ => anyhow::bail!("offer {id} needs a complete validity window at the feed or offer level"),
        };
        ensure!(
            observed_at <= now && valid_until > observed_at
                && valid_until - observed_at <= chrono::Duration::days(7),
            "invalid rate feed validity window for offer {id}"
        );
        let url = reqwest::Url::parse(&rate.source_url).context("invalid rate source URL")?;
        ensure!(
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none(),
            "invalid rate source URL"
        );
        let offer = next
            .offers
            .iter_mut()
            .find(|offer| &offer.id == id)
            .context("unreviewed offer in rate feed")?;
        // Compare with the reviewed base so reloads and fresh processes enforce the same band.
        let review_error = || format!("validate reviewed rate for offer {id}");
        match (&mut offer.pricing, &rate.schedule) {
            (
                Pricing::Image { usd_nanos_per_image: amount, .. },
                RateSchedule::Image { usd_nanos_per_image: after },
            )
            | (
                Pricing::Video { usd_nanos_per_second: amount, .. },
                RateSchedule::Video { usd_nanos_per_second: after },
            )
            | (
                Pricing::Audio { usd_nanos_per_second: amount, .. },
                RateSchedule::Audio { usd_nanos_per_second: after },
            )
            | (
                Pricing::Mesh { usd_nanos_per_mesh: amount, .. },
                RateSchedule::Mesh { usd_nanos_per_mesh: after },
            ) => {
                validate_amount(*amount, *after).with_context(review_error)?;
                *amount = *after;
            }
            (
                Pricing::Text {
                    usd_nanos_per_1000_input_tokens,
                    usd_nanos_per_1000_output_tokens,
                },
                RateSchedule::Text {
                    usd_nanos_per_1000_input_tokens: after_in,
                    usd_nanos_per_1000_output_tokens: after_out,
                },
            ) => {
                validate_amount(*usd_nanos_per_1000_input_tokens, *after_in).with_context(review_error)?;
                validate_amount(*usd_nanos_per_1000_output_tokens, *after_out).with_context(review_error)?;
                *usd_nanos_per_1000_input_tokens = *after_in;
                *usd_nanos_per_1000_output_tokens = *after_out;
            }
            _ => anyhow::bail!("rate unit changed; review the offer contract"),
        }
        let evidence = VerifiedRate { source_url: url.to_string(), observed_at, valid_until };
        if let Some(prior) = previous.rates.get(id) {
            // Copying an unchanged offer into a newer file must preserve its original deadline.
            ensure!(
                observed_at > prior.observed_at || (&evidence == prior && previous.offers.iter()
                    .find(|previous| &previous.id == id).is_some_and(|prior| prior.pricing == offer.pricing)),
                "older or conflicting rate feed for offer {id}"
            );
        }
        next.rates.insert(id.clone(), evidence);
    }
    next.revision = format!("demo_{}", digest(serde_json::to_vec(feed)?));
    Ok(next)
}

fn next_live_deadline(catalog: &Catalog, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    catalog.rates.values().map(|rate| rate.valid_until).filter(|until| *until > now).min()
}

fn validate_amount(before: i64, after: i64) -> anyhow::Result<()> {
    ensure!(
        (0..=MAX_RATE).contains(&after) && after * 2 >= before && before * 2 >= after,
        "rate change requires review"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_demo::quote::{QuoteInput, Reason, Workload};

    #[test]
    fn publishes_exact_rates_and_preserves_the_prior_snapshot_on_bad_input() {
        let base = Catalog::parse(&include_str!("../../../../../examples/connector/catalog.json").replace(r#""usd_nanos_per_image": 0"#, r#""usd_nanos_per_image": 30000000"#).into_bytes()).unwrap();
        let now = Utc::now();
        let mut feed = RateFeed {
            base_revision: base.revision.clone(),
            observed_at: Some(now),
            valid_until: Some(now + chrono::Duration::hours(1)),
            rates: BTreeMap::from([(
                "example_image".into(),
                Rate {
                    source_url: "https://vendor.example/pricing".into(),
                    observed_at: None,
                    valid_until: None,
                    schedule: RateSchedule::Image {
                        usd_nanos_per_image: 32_000_000,
                    },
                },
            )]),
        };
        let quote = QuoteInput {
            data_requirements: None,
            min_quality: None,
            min_usage_accounts: None,
            model: "example_image".into(),
            provider: None,
            preferred_providers: None,
            max_total_usd_nanos: Some(35_000_000),
            workload: Workload::Image {
                count: 1,
                aspect_ratio: "4:3".into(),
                max_relative_error_bps: Some(0),
            },
        };
        let current = apply_feed(&base, &base, &feed, now).unwrap();
        assert_ne!(base.revision, current.revision);
        assert_eq!(
            base.quote(quote.clone()).unwrap().candidates[0].estimated_usd_nanos,
            Some(30_000_000)
        );
        assert_eq!(
            current.quote(quote.clone()).unwrap().candidates[0].estimated_usd_nanos,
            Some(32_000_000)
        );
        assert_eq!(
            apply_feed(&base, &current, &feed, now)
                .unwrap()
                .revision,
            current.revision
        );

        feed.rates.get_mut("example_image").unwrap().schedule = RateSchedule::Image {
            usd_nanos_per_image: 1,
        };
        assert!(apply_feed(&base, &current, &feed, now).is_err());

        let mut expired = current;
        expired.rates.get_mut("example_image").unwrap().valid_until = now;
        let result = expired.quote(quote).unwrap();
        assert!(result.selected_offer_id.is_none());
        assert!(result
            .candidates
            .iter()
            .any(|candidate| candidate.reason == Reason::PriceStale));
        let public = serde_json::to_value(expired.model_quote(result)).unwrap();
        assert_eq!(public["candidates"][0]["rate_valid_until"], serde_json::json!(now));
        assert!(public["candidates"][0].get("rate_evidence").is_none());
    }

    #[test]
    fn bounds_successive_refreshes_to_the_reviewed_base() {
        let catalog = include_str!("../../../../../examples/connector/catalog.json")
            .replace(r#""usd_nanos_per_image": 0"#, r#""usd_nanos_per_image": 30"#);
        let base = Catalog::parse(catalog.as_bytes()).unwrap();
        let now = Utc::now();
        let feed = |amount, age| RateFeed {
            base_revision: base.revision.clone(),
            observed_at: Some(now - chrono::Duration::seconds(age)),
            valid_until: Some(now + chrono::Duration::hours(1)),
            rates: BTreeMap::from([(
                "example_image".into(),
                Rate {
                    source_url: "https://vendor.example/pricing".into(),
                    observed_at: None,
                    valid_until: None,
                    schedule: RateSchedule::Image { usd_nanos_per_image: amount },
                },
            )]),
        };
        let prior = apply_feed(&base, &base, &feed(60, 2), now).unwrap();
        assert!(apply_feed(&base, &prior, &feed(90, 1), now).is_err());
        let refreshed = apply_feed(&base, &prior, &feed(15, 1), now).unwrap();
        let restarted = apply_feed(&base, &base, &feed(15, 1), now).unwrap();
        assert_eq!(refreshed.revision, restarted.revision);
    }

    #[test]
    fn preserves_each_observation_when_only_one_offer_is_refreshed() {
        let catalog = serde_json::json!({"version": 1, "offers": [
            {"id": "first", "model": "image", "provider": "fixture", "adapter": "mock",
             "mock_latency_ms": 100, "pricing": {"kind": "image", "usd_nanos_per_image": 30, "aspect_ratios": ["4:3"]}},
            {"id": "second", "model": "image", "provider": "fixture", "adapter": "mock",
             "mock_latency_ms": 100, "pricing": {"kind": "image", "usd_nanos_per_image": 40, "aspect_ratios": ["4:3"]}}
        ]});
        let base = Catalog::parse(&serde_json::to_vec(&catalog).unwrap()).unwrap();
        let now = Utc::now();
        let observed = now - chrono::Duration::hours(2);
        let expired = now - chrono::Duration::hours(1);
        let fresh = now + chrono::Duration::hours(1);
        let mut feed: RateFeed = serde_json::from_value(serde_json::json!({
            "base_revision": base.revision,
            "rates": {
                "first": {"source_url": "https://vendor.example/pricing", "observed_at": observed,
                          "valid_until": expired, "schedule": {"kind": "image", "usd_nanos_per_image": 30}},
                "second": {"source_url": "https://vendor.example/pricing", "observed_at": observed,
                           "valid_until": fresh, "schedule": {"kind": "image", "usd_nanos_per_image": 40}}
            }
        })).unwrap();
        let prior = apply_feed(&base, &base, &feed, now).unwrap();
        let second = feed.rates.get_mut("second").unwrap();
        second.observed_at = Some(now);
        second.schedule = RateSchedule::Image { usd_nanos_per_image: 35 };
        let next = apply_feed(&base, &prior, &feed, now).unwrap();
        assert!(next.expired_offer("first", now));
        assert_eq!(next.rates["first"], prior.rates["first"]);
        assert_eq!(next_live_deadline(&next, now), Some(fresh));
        assert!(apply_feed(&base, &next, &feed, now).is_ok());

        feed.rates.get_mut("first").unwrap().valid_until = Some(fresh);
        assert!(apply_feed(&base, &next, &feed, now).err().unwrap().to_string().contains("conflicting rate feed for offer first"));
        feed.rates.get_mut("first").unwrap().valid_until = None;
        assert!(apply_feed(&base, &next, &feed, now).err().unwrap().to_string().contains("complete validity window"));
    }

    #[test]
    fn refreshes_media_rates_without_changing_capabilities() {
        let offers = serde_json::json!({"version": 1, "offers": [
            {"id": "video", "model": "media", "provider": "fixture", "adapter": "mock",
             "mock_latency_ms": 100, "pricing": {"kind": "video", "usd_nanos_per_second": 20,
             "durations_seconds": [5], "aspect_ratios": ["16:9"], "resolution": "720p"}},
            {"id": "audio", "model": "media", "provider": "fixture", "adapter": "mock",
             "mock_latency_ms": 100, "pricing": {"kind": "audio", "usd_nanos_per_second": 12,
             "durations_seconds": [10], "format": "wav"}},
            {"id": "mesh", "model": "media", "provider": "fixture", "adapter": "mock",
             "mock_latency_ms": 100, "pricing": {"kind": "mesh", "usd_nanos_per_mesh": 40,
             "format": "glb"}}
        ]});
        let base = Catalog::parse(&serde_json::to_vec(&offers).unwrap()).unwrap();
        let now = Utc::now();
        let mut feed: RateFeed = serde_json::from_value(serde_json::json!({
            "base_revision": base.revision, "observed_at": now,
            "valid_until": now + chrono::Duration::minutes(5),
            "rates": {
                "video": {"source_url": "https://vendor.example/prices",
                          "schedule": {"kind": "video", "usd_nanos_per_second": 25}},
                "audio": {"source_url": "https://vendor.example/prices",
                          "schedule": {"kind": "audio", "usd_nanos_per_second": 15}},
                "mesh": {"source_url": "https://vendor.example/prices",
                         "schedule": {"kind": "mesh", "usd_nanos_per_mesh": 44}}
            }
        })).unwrap();

        let next = apply_feed(&base, &base, &feed, now).unwrap();
        assert_eq!(next.offers[0].pricing, Pricing::Audio {
            usd_nanos_per_second: 15, durations_seconds: vec![10], format: "wav".into(),
        });
        assert_eq!(next.offers[1].pricing, Pricing::Mesh {
            usd_nanos_per_mesh: 44, format: "glb".into(),
        });
        assert_eq!(next.offers[2].pricing, Pricing::Video {
            usd_nanos_per_second: 25, durations_seconds: vec![5],
            aspect_ratios: vec!["16:9".into()], resolution: "720p".into(),
        });

        feed.rates.get_mut("audio").unwrap().schedule =
            RateSchedule::Video { usd_nanos_per_second: 15 };
        assert!(apply_feed(&base, &next, &feed, now).is_err());
    }
}
