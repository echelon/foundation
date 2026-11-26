use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::ensure;
use ids::AccountId;
use reqwest::{Url, header::HeaderValue};
use serde::{Deserialize, Serialize};

use super::super::chat_transport::authorization;
use super::{Binding, ChatClient, bearer_key, digest, fixture_endpoint};
use crate::gateway_demo::catalog::valid_identifier;
use crate::gateway_demo::compute_evidence::{ComputeOutcome, ComputeSelection};

const CHAT_PATH: &str = "/v1/chat/completions";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::gateway_demo) struct ComputeBinding {
    pub name: String,
    allowed_locations: BTreeSet<Location>,
    targets: Vec<EndpointConfig>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(in crate::gateway_demo) struct Location {
    pub cloud: String,
    pub region: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::gateway_demo) struct EndpointConfig {
    pub id: String,
    pub location: Location,
    pub zone: String,
    pub priority: u8,
    pub endpoint: String,
    pub key_env: Option<String>,
    #[serde(default)]
    pub retry_on_rate_limit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<AccountId>,
}

#[derive(Serialize)]
pub(in crate::gateway_demo) struct TargetView {
    pub id: String,
    cloud: String,
    region: String,
    zone: String,
    priority: u8,
}

pub(in crate::gateway_demo) struct ComputeStack {
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
    pub(super) fn is_organization_scoped(&self) -> bool {
        !self.targets.is_empty() && self.targets.iter().all(|target| target.organization_id.is_some())
    }

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
            let endpoint = config.validate()?;
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

impl EndpointConfig {
    pub(in crate::gateway_demo) fn validate(&self) -> anyhow::Result<Url> {
        ensure!(valid_identifier(&self.id) && valid_identifier(&self.location.cloud)
            && valid_identifier(&self.location.region) && valid_identifier(&self.zone),
            "enter valid target, cloud, region, and zone IDs");
        ensure!(self.endpoint.len() <= 2048, "compute endpoint is too long");
        if let Some(key) = &self.key_env {
            super::validate_key_env(key)?;
        }
        compute_endpoint(self)
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

    pub(in crate::gateway_demo) fn target_views(&self, account: &AccountId) -> Vec<TargetView> {
        let mut targets: Vec<_> = self.targets.iter().filter(|target| target.visible(Some(account)))
            .map(|target| TargetView {
                id: target.config.id.clone(),
                cloud: target.config.location.cloud.clone(),
                region: target.config.location.region.clone(),
                zone: target.config.zone.clone(),
                priority: target.config.priority,
            }).collect();
        targets.sort_by(|left, right| left.priority.cmp(&right.priority).then(left.id.cmp(&right.id)));
        targets
    }

    pub(in crate::gateway_demo) fn has_owned_target(&self, account: &AccountId) -> bool {
        self.targets.iter().any(|target| target.config.organization_id.as_ref() == Some(account))
    }

    pub(in crate::gateway_demo) fn review_addition(&self, config: &EndpointConfig) -> anyhow::Result<()> {
        ensure!(self.targets.len() < 4, "this pool already has four targets");
        ensure!(!self.targets.iter().any(|target| target.config.id == config.id
            || target.config.endpoint == config.endpoint), "target ID or endpoint is already configured");
        Ok(())
    }

    pub(in crate::gateway_demo) fn has_target(&self, account: &AccountId, order: Option<&[String]>) -> bool {
        self.targets.iter().any(|target| target.visible(Some(account))
            && order.is_none_or(|ids| ids.contains(&target.config.id)))
    }

    pub(in crate::gateway_demo::providers) fn ordered_targets(
        &self, request_id: &str, account: Option<&AccountId>, order: Option<&[String]>,
    ) -> Vec<&Target> {
        let visible = || self.targets.iter().filter(|target| target.visible(account));
        if let Some(order) = order {
            return order.iter().filter_map(|id| visible().find(|target| target.config.id == *id)).collect();
        }
        let mut targets: Vec<_> = visible().collect();
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
    fn visible(&self, account: Option<&AccountId>) -> bool {
        self.config.organization_id.as_ref().is_none_or(|owner| Some(owner) == account)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_order_filters_and_reorders_only_targets_visible_to_its_account() {
        let owner = AccountId::generate();
        let sibling = AccountId::generate();
        let binding: ComputeBinding = serde_json::from_value(serde_json::json!({
            "name": "pool",
            "allowed_locations": [{"cloud":"aws","region":"east"}],
            "targets": [
                {"id":"a","location":{"cloud":"aws","region":"east"},"zone":"one","priority":10,
                 "endpoint":"http://127.0.0.1:9811/v1/chat/completions"},
                {"id":"b","location":{"cloud":"aws","region":"east"},"zone":"two","priority":20,
                 "endpoint":"http://127.0.0.1:9812/v1/chat/completions"},
                {"id":"private","location":{"cloud":"aws","region":"east"},"zone":"three","priority":30,
                 "endpoint":"http://127.0.0.1:9813/v1/chat/completions","organization_id":owner}
            ]
        })).unwrap();
        let binding = binding.build(Some("sample")).unwrap();
        let stack = binding.client.compute_stack().unwrap();
        let custom = vec!["private".into(), "b".into()];
        let ids = |account, order| stack.ordered_targets("request", Some(account), order)
            .into_iter().map(|target| target.config.id.as_str()).collect::<Vec<_>>();
        assert_eq!(ids(&owner, Some(&custom)), ["private", "b"]);
        assert_eq!(ids(&sibling, Some(&custom)), ["b"]);
        assert_eq!(ids(&sibling, None), ["a", "b"]);
        assert!(!stack.has_target(&sibling, Some(&custom[..1])));
    }
}
