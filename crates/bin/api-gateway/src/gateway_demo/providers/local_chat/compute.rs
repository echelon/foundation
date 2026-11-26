use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::ensure;
use reqwest::{Url, header::HeaderValue};
use serde::{Deserialize, Serialize};

use super::super::chat_transport::authorization;
use super::{Binding, ChatClient, bearer_key, digest, fixture_endpoint};
use crate::gateway_demo::catalog::valid_identifier;
use crate::gateway_demo::compute_evidence::{ComputeOutcome, ComputeSelection};

const CHAT_PATH: &str = "/v1/chat/completions";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComputeBinding {
    pub name: String,
    allowed_locations: BTreeSet<Location>,
    targets: Vec<EndpointConfig>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Location {
    cloud: String,
    region: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EndpointConfig {
    id: String,
    location: Location,
    zone: String,
    priority: u8,
    endpoint: String,
    key_env: Option<String>,
    #[serde(default)]
    retry_on_rate_limit: bool,
}

pub(in crate::gateway_demo::providers) struct ComputeStack {
    pub cell: String,
    pub name: String,
    targets: Vec<Target>,
}

pub(in crate::gateway_demo::providers) struct Target {
    config: EndpointConfig,
    pub endpoint: Url,
    pub auth: Option<HeaderValue>,
    unavailable_until: Mutex<Option<Instant>>,
}

impl ComputeBinding {
    pub(super) fn build(self, cell: Option<&str>) -> anyhow::Result<Binding<ChatClient>> {
        let cell = cell
            .filter(|cell| valid_identifier(cell))
            .ok_or_else(|| anyhow::anyhow!("compute stacks require a valid cell ID"))?;
        ensure!(
            (1..=4).contains(&self.targets.len())
                && (1..=4).contains(&self.allowed_locations.len()),
            "a compute stack requires one to four targets and allowed locations"
        );
        ensure!(
            self.allowed_locations.iter().all(|location| {
                valid_identifier(&location.cloud) && valid_identifier(&location.region)
            }),
            "compute locations require valid cloud and region IDs"
        );

        let mut ids = BTreeSet::new();
        let mut endpoints = BTreeSet::new();
        let mut targets = Vec::with_capacity(self.targets.len());
        for mut config in self.targets {
            ensure!(
                valid_identifier(&config.id)
                    && valid_identifier(&config.zone)
                    && ids.insert(config.id.clone()),
                "each compute target requires a unique target ID and a valid zone ID"
            );
            ensure!(
                self.allowed_locations.contains(&config.location),
                "compute target is outside the stack's allowed locations"
            );
            let endpoint = compute_endpoint(&config)?;
            config.endpoint = endpoint.to_string();
            ensure!(
                endpoints.insert(config.endpoint.clone()),
                "duplicate compute endpoint"
            );
            let auth = config
                .key_env
                .as_deref()
                .map(bearer_key)
                .transpose()?
                .as_deref()
                .map(authorization)
                .transpose()?;
            targets.push(Target {
                config,
                endpoint,
                auth,
                unavailable_until: Mutex::new(None),
            });
        }
        targets.sort_by(|a, b| a.config.id.cmp(&b.config.id));
        let topology: Vec<_> = targets.iter().map(|target| &target.config).collect();
        let digest = digest(serde_json::to_vec(&(
            "compute-chat",
            cell,
            &self.name,
            &self.allowed_locations,
            topology,
        ))?);
        let client = ChatClient::compute(ComputeStack {
            cell: cell.to_owned(),
            name: self.name,
            targets,
        })?;
        Ok(Binding { digest, client })
    }
}

fn compute_endpoint(config: &EndpointConfig) -> anyhow::Result<Url> {
    if let Ok(endpoint) = fixture_endpoint(&config.endpoint, CHAT_PATH) {
        return Ok(endpoint);
    }
    let endpoint =
        Url::parse(&config.endpoint).map_err(|_| anyhow::anyhow!("invalid compute target URL"))?;
    ensure!(
        endpoint.scheme() == "https"
            && endpoint.host().is_some()
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none()
            && endpoint.path() == CHAT_PATH
            && config.key_env.is_some(),
        "remote compute requires HTTPS, a bearer key reference, and the exact chat path"
    );
    Ok(endpoint)
}

impl ComputeStack {
    pub fn covers_regions(&self, declared: &[String]) -> bool {
        self.targets.iter().all(|target| declared.contains(&target.config.location.region))
    }

    pub fn ordered_targets(&self, request_id: &str) -> Vec<&Target> {
        let mut targets: Vec<_> = self.targets.iter().collect();
        targets.sort_by_cached_key(|target| {
            (
                target.config.priority,
                digest(format!("{request_id}/{}", target.config.id)),
            )
        });
        targets
    }
}

impl Target {
    pub fn retries_rate_limit(&self) -> bool {
        self.config.retry_on_rate_limit
    }

    pub fn available(&self, now: Instant) -> bool {
        self.unavailable_until
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_none_or(|until| until <= now)
    }

    pub fn cool_down_until(&self, until: Instant) {
        let mut current = self
            .unavailable_until
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *current = Some(current.map_or(until, |previous| previous.max(until)));
    }

    pub fn selection(&self, stack: &ComputeStack) -> ComputeSelection {
        ComputeSelection {
            home_cell: stack.cell.clone(),
            stack: stack.name.clone(),
            target: self.config.id.clone(),
            cloud: self.config.location.cloud.clone(),
            region: self.config.location.region.clone(),
            zone: self.config.zone.clone(),
            priority: self.config.priority,
        }
    }

    pub fn record(&self, stack: &ComputeStack, outcome: ComputeOutcome) {
        tracing::info!(
            cell = %stack.cell, stack = %stack.name, target = %self.config.id,
            cloud = %self.config.location.cloud, region = %self.config.location.region,
            zone = %self.config.zone, outcome = outcome.code(), "compute target submission"
        );
    }
}
