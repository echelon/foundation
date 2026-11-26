use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use anyhow::{Context, ensure};
use reqwest::Url;
use serde::Deserialize;

use super::{Binding, ChatClient, fixture_endpoint};
use crate::gateway_demo::catalog::{Adapter, Catalog, digest};

pub(super) mod compute;
use compute::ComputeBinding;

#[cfg(test)]
mod tests;

const CONFIG: &str = "TOKENCRAFT_LOCAL_CHAT_BINDINGS";
const KEY_PREFIX: &str = "TOKENCRAFT_LOCAL_CHAT_KEY_";
const MAX_BYTES: u64 = 16 * 1024;
pub(in crate::gateway_demo) const MAX_BINDINGS: usize = 16;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    cell: Option<String>,
    bindings: Vec<BindingConfig>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BindingConfig {
    Local(LocalBinding),
    Compute(ComputeBinding),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalBinding {
    name: String,
    endpoint: String,
    key_env: Option<String>,
    #[serde(default)]
    tenant_keys: bool,
    revision: Option<String>,
}

pub(super) fn load(catalog: &Catalog) -> anyhow::Result<BTreeMap<String, Binding<ChatClient>>> {
    let names: BTreeSet<_> = catalog
        .offers
        .iter()
        .filter(|offer| offer.adapter == Adapter::OpenAiCompatibleChat)
        .filter_map(|offer| offer.local_binding.as_deref())
        .collect();
    let path = std::env::var_os(CONFIG);
    if names.is_empty() {
        ensure!(
            path.is_none(),
            "local chat bindings require a self-hosted chat offer"
        );
        return Ok(BTreeMap::new());
    }
    let path = path.context("set TOKENCRAFT_LOCAL_CHAT_BINDINGS for self-hosted chat offers")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .context("open local chat bindings")?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "local chat bindings exceed 16 KiB"
    );
    parse(&bytes, &names)
}

fn parse(
    bytes: &[u8],
    names: &BTreeSet<&str>,
) -> anyhow::Result<BTreeMap<String, Binding<ChatClient>>> {
    let config: Config = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("invalid local chat binding configuration"))?;
    ensure!(
        config.version == 1 && (1..=MAX_BINDINGS).contains(&config.bindings.len()),
        "local chat requires version 1 and one to 16 bindings"
    );
    let mut bindings = BTreeMap::new();
    for entry in config.bindings {
        let name = match &entry {
            BindingConfig::Local(local) => &local.name,
            BindingConfig::Compute(compute) => &compute.name,
        };
        ensure!(
            names.contains(name.as_str()) && !bindings.contains_key(name),
            "each local chat binding must be named by a catalog offer exactly once"
        );
        let name = name.to_owned();
        let binding = match entry {
            BindingConfig::Local(local) => local.build(),
            BindingConfig::Compute(compute) => compute.build(config.cell.as_deref()),
        }.with_context(|| format!("configure chat binding {name}"))?;
        bindings.insert(name, binding);
    }
    if let Some(name) = names.iter().find(|name| !bindings.contains_key(**name)) {
        anyhow::bail!("a self-hosted offer requires chat binding {name}");
    }
    Ok(bindings)
}

impl LocalBinding {
    fn build(&self) -> anyhow::Result<Binding<ChatClient>> {
        let path = "/v1/chat/completions";
        let endpoint = match fixture_endpoint(&self.endpoint, path) {
            Ok(endpoint) => endpoint,
            Err(_) if self.tenant_keys => tenant_https_endpoint(&self.endpoint)?,
            Err(_) => anyhow::bail!("local chat endpoints require literal loopback HTTP and {path}"),
        };
        let token = self.key_env.as_deref().map(bearer_key).transpose()?;
        let identity = format!("{}/v1/{}/{endpoint}", Adapter::OpenAiCompatibleChat.as_str(), self.name);
        let (client, digest) = if self.tenant_keys {
            let revision = self.revision.as_deref()
                .filter(|revision| crate::gateway_demo::catalog::valid_identifier(revision))
                .context("tenant chat bindings need a revision")?;
            let token = token.as_deref().filter(|token| token.len() >= 32)
                .context("tenant chat bindings need an independent service bearer key of at least 32 characters")?;
            let digest = digest(format!("{identity}/tenant_keys/{revision}"));
            (ChatClient::tenant(endpoint, token)?, digest)
        } else {
            ensure!(self.revision.is_none(), "a chat binding revision requires tenant_keys");
            (ChatClient::local(endpoint, token.as_deref())?, digest(identity))
        };
        Ok(Binding { digest, client })
    }
}

fn tenant_https_endpoint(raw: &str) -> anyhow::Result<Url> {
    let endpoint = Url::parse(raw).map_err(|_| anyhow::anyhow!("invalid tenant chat connector URL"))?;
    ensure!(endpoint.scheme() == "https" && endpoint.host_str().is_some()
        && endpoint.path() == "/v1/chat/completions" && endpoint.username().is_empty()
        && endpoint.password().is_none() && endpoint.query().is_none() && endpoint.fragment().is_none(),
        "tenant chat connectors require HTTPS and /v1/chat/completions");
    Ok(endpoint)
}

pub(in crate::gateway_demo) fn validate_key_env(name: &str) -> anyhow::Result<()> {
    ensure!(
        name.len() <= 96
            && name
                .strip_prefix(KEY_PREFIX)
                .is_some_and(|suffix| !suffix.is_empty()
                    && suffix.bytes().all(|byte| byte.is_ascii_uppercase()
                        || byte.is_ascii_digit()
                        || byte == b'_')),
        "local chat keys must reference a TOKENCRAFT_LOCAL_CHAT_KEY_ environment variable"
    );
    Ok(())
}

fn bearer_key(name: &str) -> anyhow::Result<String> {
    validate_key_env(name)?;
    let value = std::env::var(name).map_err(|_| anyhow::anyhow!("set {name} for local chat"))?;
    ensure!(
        (1..=256).contains(&value.len())
            && value.bytes().all(|byte| byte.is_ascii_alphanumeric()
                || matches!(byte, b'_' | b'-' | b'.' | b'~' | b'+' | b'/' | b'=')),
        "invalid local chat bearer key"
    );
    Ok(value)
}
