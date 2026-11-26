use std::{collections::BTreeSet, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use db_queries::gateway_requests::RequestScope;
use ids::{AccountId, GatewayProjectId};
use serde::{Deserialize, Serialize};

use super::{
    catalog::{valid_identifier, Adapter, Catalog, CatalogFile, Offer, Pricing, Provider},
    catalog_refresh::CatalogStore,
    projects::{managed_project, OrganizationQuery},
    providers::{connector, fixture_endpoint, local_chat, ProviderRuntime},
    provider_access::{self, CatalogAccess},
    routing_policy::Policies,
    routing_settings,
};
use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Serialize)]
pub(super) struct Inventory {
    base_revision: String,
    active_revision: String,
    catalog_revision: String,
    offers: Vec<ActiveOffer>,
    policy_revision: i32,
    discovery: bool,
    provider_key_management: bool,
    credential_bindings: Vec<String>,
}

#[derive(Serialize)]
struct ActiveOffer {
    id: String,
    provider: Provider,
    model: String,
    adapter: Adapter,
    binding: Option<String>,
    pricing: Pricing,
    reviewed_pricing: Pricing,
    allowed_by_policy: bool,
    connection_id: String,
    selected_projects: Option<Vec<GatewayProjectId>>,
    available_in_project: bool,
    excluded_in_project: bool,
}

pub(super) async fn inventory(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw_project): Path<String>,
) -> ApiResult<Json<Inventory>> {
    let (account, project) = managed_project(&state, &auth, &query, &raw_project).await?;
    let scope = RequestScope {
        account: &account,
        project: &project.id,
        client_ref: None,
    };
    let policies = Policies::load(&state.pool, scope).await?;
    build_inventory(store.as_ref(), runtime.as_deref(), &account, Some(scope), policies)
}

pub(super) async fn organization_inventory(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
) -> ApiResult<Json<Inventory>> {
    let account = provider_access::managed_account(&state, &auth, &query).await?;
    let mut policies = Policies::default();
    policies.organization = routing_settings::current(&state.pool, &account, None).await?;
    build_inventory(store.as_ref(), runtime.as_deref(), &account, None, policies)
}

fn build_inventory(
    store: &CatalogStore,
    runtime: Option<&ProviderRuntime>,
    account: &AccountId,
    scope: Option<RequestScope<'_>>,
    policies: Policies,
) -> ApiResult<Json<Inventory>> {
    let reviewed = store.reviewed();
    let active = store.current();
    let offers: Vec<_> = reviewed
        .offers
        .iter()
        .filter(|offer| reviewed.visible_to(offer, Some(account)))
        .filter_map(|offer| {
            let connection_id = offer.connection_id();
            let selected = policies.connection_projects(reviewed, &connection_id);
            let available = scope.is_none_or(|scope| selected.is_none_or(|projects| projects.contains(scope.project)));
            let excluded = policies.project.policy.excluded_connections.contains(&connection_id);
            let current = active.offers.iter().find(|active| active.id == offer.id)?;
            Some(ActiveOffer {
                id: offer.id.clone(),
                provider: offer.provider.clone(),
                model: offer.model.clone(),
                adapter: offer.adapter,
                binding: offer.local_binding.clone(),
                pricing: current.pricing.clone(),
                reviewed_pricing: offer.pricing.clone(),
                allowed_by_policy: policies.allows_provider(&offer.provider) && available && !excluded,
                selected_projects: selected.map(|projects| projects.iter().cloned().collect()),
                available_in_project: available,
                excluded_in_project: excluded,
                connection_id,
            })
        })
        .collect();
    let credential_bindings = offers.iter()
        .filter_map(|offer| {
            if !offer.available_in_project || offer.excluded_in_project {
                return None;
            }
            let binding = offer.binding.as_deref()?;
            let (adapter, _) = runtime?.tenant_binding(binding)?;
            (adapter == offer.adapter).then_some(binding)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect();
    Ok(Json(Inventory {
        base_revision: reviewed.revision.clone(),
        active_revision: active.revision.clone(),
        catalog_revision: reviewed.file_revision.clone(),
        offers,
        policy_revision: if scope.is_some() { policies.project.revision } else { policies.organization.revision },
        discovery: false,
        provider_key_management: false,
        credential_bindings,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreviewInput {
    base_revision: String,
    change: Change,
    projects: Option<std::collections::BTreeSet<GatewayProjectId>>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Attach {
        name: String,
        provider_id: Option<Provider>,
        binding: Binding,
        models: Vec<Model>,
    },
    AddModels {
        binding_name: String,
        models: Vec<Model>,
    },
    UpdatePrice {
        offer_id: String,
        primary_usd_nanos: i64,
        output_usd_nanos: Option<i64>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Binding {
    Connector {
        name: String,
        origin: String,
        revision: String,
        token_env: String,
    },
    Chat {
        name: String,
        endpoint: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        key_env: Option<String>,
    },
}

impl Binding {
    fn validate(&self) -> anyhow::Result<(&str, Adapter)> {
        match self {
            Self::Connector {
                name,
                origin,
                revision,
                token_env,
            } => {
                anyhow::ensure!(origin.len() <= 2048, "connector origin is too long");
                let url = reqwest::Url::parse(origin)
                    .map_err(|_| anyhow::anyhow!("enter a valid connector origin"))?;
                anyhow::ensure!(
                    valid_identifier(name) && valid_identifier(revision) && connector::valid_origin(&url),
                    "use a valid binding, revision and HTTPS origin or literal loopback HTTP origin"
                );
                connector::validate_token_env(token_env)?;
                Ok((name, Adapter::ConnectorQueue))
            }
            Self::Chat {
                name,
                endpoint,
                key_env,
            } => {
                anyhow::ensure!(
                    valid_identifier(name) && endpoint.len() <= 2048,
                    "invalid chat binding name or endpoint length"
                );
                fixture_endpoint(endpoint, "/v1/chat/completions").map_err(|_| {
                    anyhow::anyhow!("use literal loopback HTTP ending in /v1/chat/completions")
                })?;
                if let Some(key) = key_env {
                    local_chat::validate_key_env(key)?;
                }
                Ok((name, Adapter::OpenAiCompatibleChat))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    id: String,
    model: String,
    upstream_model: String,
    pricing: Pricing,
}

#[derive(Serialize)]
pub(super) struct Preview {
    base_revision: String,
    base_catalog_revision: String,
    proposed_revision: String,
    provider_name: Option<String>,
    catalog: CatalogFile,
    binding: Option<Binding>,
    changed_offer_ids: Vec<String>,
    rate_feed_active: bool,
    catalog_patch: bool,
}

#[derive(Serialize)]
pub(super) struct ProjectPreview {
    project_id: GatewayProjectId,
    #[serde(flatten)]
    preview: Preview,
}

pub(super) async fn preview(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw_project): Path<String>,
    Json(input): Json<PreviewInput>,
) -> ApiResult<Json<ProjectPreview>> {
    let (account, project) = managed_project(&state, &auth, &query, &raw_project).await?;
    let result = build_preview(&store, runtime.as_deref(), account, input, false)?;
    Ok(Json(ProjectPreview { project_id: project.id, preview: result }))
}

pub(super) async fn organization_preview(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Json(input): Json<PreviewInput>,
) -> ApiResult<Json<Preview>> {
    let account = provider_access::managed_account(&state, &auth, &query).await?;
    provider_access::validate_projects(&state, &account, input.projects.as_ref()).await?;
    build_preview(&store, runtime.as_deref(), account, input, true).map(Json)
}

fn build_preview(
    store: &CatalogStore,
    runtime: Option<&ProviderRuntime>,
    account: AccountId,
    input: PreviewInput,
    organization: bool,
) -> ApiResult<Preview> {
    let reviewed = store.reviewed();
    if input.base_revision != reviewed.revision {
        return Err(ApiError::Conflict(
            "The reviewed catalog changed. Refresh providers and review the change again.".into(),
        ));
    }
    let connector_names: Vec<_> = runtime.into_iter()
        .flat_map(ProviderRuntime::connector_binding_names)
        .collect();
    let attach = matches!(&input.change, Change::Attach { .. });
    let mut preview = plan(
        reviewed, &store.current().revision, &connector_names, input.change,
    ).map_err(|error| ApiError::BadRequest(error.to_string()))?;

    for offer in preview.catalog.offers.iter().filter(|offer| preview.changed_offer_ids.contains(&offer.id)) {
        if !reviewed.visible_to(offer, Some(&account)) {
            return Err(ApiError::NotFound("provider connection".into()));
        }
    }

    if attach && organization {
        let id = preview.catalog.offers.iter()
            .find(|offer| preview.changed_offer_ids.contains(&offer.id))
            .ok_or_else(|| ApiError::Internal("proposed connection has no offers".into()))?
            .connection_id();
        preview.catalog.connection_access.insert(id, CatalogAccess {
            account: account.clone(), projects: input.projects,
        });
        preview.proposed_revision = Catalog::parse(&serde_json::to_vec(&preview.catalog)
            .map_err(|_| ApiError::Internal("could not encode catalog".into()))?)
            .map_err(|error| ApiError::BadRequest(error.to_string()))?.revision;
    }

    // A tenant can download a patch without receiving another tenant's catalog entries.
    if organization || reviewed.connection_access.values().any(|access| access.account != account) {
        preview.catalog_patch = true;
        preview.catalog.offers.retain(|offer| preview.changed_offer_ids.contains(&offer.id));
        preview.catalog.connection_access.retain(|id, _| preview.catalog.offers.iter().any(|offer| offer.connection_id() == *id));
        preview.catalog.rate_feed_offers = None;
    }
    Ok(preview)
}

fn plan(
    base: &Catalog,
    active_revision: &str,
    connector_names: &[&str],
    change: Change,
) -> anyhow::Result<Preview> {
    let mut offers = base.offers.clone();
    let (provider_name, binding, changed_offer_ids) = match change {
        Change::Attach {
            name,
            provider_id,
            binding,
            models,
        } => {
            let display = name.trim();
            anyhow::ensure!(
                !display.is_empty()
                    && display.chars().count() <= 80
                    && !display.chars().any(char::is_control),
                "enter a provider name up to 80 characters"
            );
            let (binding_name, adapter) = binding.validate()?;
            anyhow::ensure!(
                !offers
                    .iter()
                    .any(|offer| offer.local_binding.as_deref() == Some(binding_name)),
                "that binding name is in use; choose a new one"
            );
            if adapter == Adapter::ConnectorQueue {
                anyhow::ensure!(
                    !connector_names.contains(&binding_name),
                    "that connector binding is still configured; choose a new name"
                );
                anyhow::ensure!(
                    connector_names.len() < connector::MAX_BINDINGS,
                    "the connector registry already has 64 bindings"
                );
            }
            if adapter == Adapter::OpenAiCompatibleChat {
                let bindings: std::collections::HashSet<_> = offers
                    .iter()
                    .filter(|offer| offer.adapter == adapter)
                    .filter_map(|offer| offer.local_binding.as_deref())
                    .collect();
                anyhow::ensure!(
                    bindings.len() < local_chat::MAX_BINDINGS,
                    "the chat registry already has 16 bindings"
                );
            }
            let provider_id = provider_id
                .unwrap_or_else(|| "self_hosted".parse().expect("valid default provider ID"));
            let changed = add_models(&mut offers, binding_name, adapter, provider_id, models)?;
            (Some(display.to_owned()), Some(binding), changed)
        }
        Change::AddModels {
            binding_name,
            models,
        } => {
            let mut identities = offers
                .iter()
                .filter(|offer| offer.local_binding.as_deref() == Some(binding_name.as_str()))
                .map(|offer| (offer.adapter, &offer.provider));
            let (adapter, provider) = identities.next().ok_or_else(|| {
                anyhow::anyhow!("select an attached binding in the reviewed catalog")
            })?;
            anyhow::ensure!(
                identities.all(|candidate| candidate == (adapter, provider)),
                "that binding is shared by different providers or connection types"
            );
            let provider = provider.clone();
            let changed = add_models(&mut offers, &binding_name, adapter, provider, models)?;
            (None, None, changed)
        }
        Change::UpdatePrice {
            offer_id,
            primary_usd_nanos,
            output_usd_nanos,
        } => {
            let offer = offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
                .ok_or_else(|| anyhow::anyhow!("select an offer in the reviewed catalog"))?;
            let previous = offer.pricing.clone();
            set_price(&mut offer.pricing, primary_usd_nanos, output_usd_nanos)?;
            anyhow::ensure!(
                offer.pricing != previous,
                "enter a price that differs from the reviewed catalog"
            );
            (None, None, vec![offer_id])
        }
    };
    let candidate = Catalog::parse(&serde_json::to_vec(&CatalogFile {
        version: 1, offers, connection_access: base.connection_access.clone(), rate_feed_offers: base.rate_feed_offers.clone(),
    })?)?;
    Ok(Preview {
        base_revision: base.revision.clone(),
        base_catalog_revision: base.file_revision.clone(),
        proposed_revision: candidate.revision.clone(),
        provider_name,
        catalog: candidate.into_file(),
        binding,
        changed_offer_ids,
        rate_feed_active: active_revision != base.revision,
        catalog_patch: false,
    })
}

fn add_models(
    offers: &mut Vec<Offer>,
    binding: &str,
    adapter: Adapter,
    provider: Provider,
    models: Vec<Model>,
) -> anyhow::Result<Vec<String>> {
    anyhow::ensure!(
        !models.is_empty() && models.len() <= 24,
        "include one to 24 models"
    );
    let changed = models.iter().map(|model| model.id.clone()).collect();
    offers.extend(models.into_iter().map(|model| Offer {
        id: model.id,
        provider: provider.clone(),
        model: model.model,
        adapter,
        mock_latency_ms: None,
        upstream_model: Some(model.upstream_model),
        local_binding: Some(binding.to_owned()),
        data_handling: None,
        artcraft_reference: None,
        pricing: model.pricing,
    }));
    Ok(changed)
}

fn set_price(pricing: &mut Pricing, primary: i64, output: Option<i64>) -> anyhow::Result<()> {
    let rate = match pricing {
        Pricing::Text {
            usd_nanos_per_1000_input_tokens,
            usd_nanos_per_1000_output_tokens,
        } => {
            *usd_nanos_per_1000_input_tokens = primary;
            *usd_nanos_per_1000_output_tokens =
                output.ok_or_else(|| anyhow::anyhow!("enter an output token price"))?;
            return Ok(());
        }
        Pricing::Image {
            usd_nanos_per_image,
            ..
        } => usd_nanos_per_image,
        Pricing::Video {
            usd_nanos_per_second,
            ..
        }
        | Pricing::Audio {
            usd_nanos_per_second,
            ..
        } => usd_nanos_per_second,
        Pricing::Mesh {
            usd_nanos_per_mesh, ..
        } => usd_nanos_per_mesh,
    };
    anyhow::ensure!(
        output.is_none(),
        "an output token price is only used for chat"
    );
    *rate = primary;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Catalog {
        Catalog::parse(include_bytes!(
            "../../../../../examples/connector/catalog.json"
        ))
        .unwrap()
    }

    #[test]
    fn price_changes_preserve_the_binding_and_other_offer_fields() {
        let mut base = fixture();
        let file_revision = base.revision.clone();
        base.revision = "pinned-to-binding".into();
        let change = Change::UpdatePrice {
            offer_id: "example_image".into(),
            primary_usd_nanos: 25_000_000,
            output_usd_nanos: None,
        };
        let preview = plan(&base, "active-rate-feed", &[], change).unwrap();
        assert!(preview.rate_feed_active);
        assert_eq!(preview.base_revision, base.revision);
        assert_eq!(preview.base_catalog_revision, file_revision);
        assert_ne!(preview.proposed_revision, file_revision);
        assert!(preview.binding.is_none());
        let offer = &preview.catalog.offers[0];
        assert_eq!(offer.local_binding.as_deref(), Some("example"));
        assert_eq!(offer.upstream_model.as_deref(), Some("example-image"));
        assert_eq!(
            offer.pricing,
            Pricing::Image {
                usd_nanos_per_image: 25_000_000,
                aspect_ratios: vec!["1:1".into(), "4:3".into()]
            }
        );
    }

    #[test]
    fn attaching_and_adding_models_use_catalog_validation_without_reading_secrets() {
        let base = fixture();
        let model = |id: &str| Model {
            id: id.into(),
            model: id.into(),
            upstream_model: "vendor/video".into(),
            pricing: Pricing::Video {
                usd_nanos_per_second: 10_000_000,
                durations_seconds: vec![5],
                aspect_ratios: vec!["16:9".into()],
                resolution: "720p".into(),
            },
        };
        let attach = |origin: &str| Change::Attach {
            name: "New Connector".into(),
            provider_id: Some("my-provider".parse().unwrap()),
            binding: Binding::Connector {
                name: "new".into(),
                revision: "v1".into(),
                origin: origin.into(),
                token_env: "TOKENCRAFT_CONNECTOR_TOKEN_NEW".into(),
            },
            models: vec![model("new_video")],
        };
        let preview = plan(
            &base,
            &base.revision,
            &[],
            attach("https://connector.example.com/"),
        )
        .unwrap();
        assert_eq!(preview.catalog.offers.len(), 2);
        assert_eq!(preview.catalog.offers[1].provider.as_str(), "my-provider");
        assert!(matches!(preview.binding, Some(Binding::Connector { .. })));
        assert!(plan(
            &base,
            &base.revision,
            &[],
            attach("https://connector.example.com/path")
        )
        .is_err());
        assert!(plan(
            &base,
            &base.revision,
            &["new"],
            attach("https://connector.example.com/"),
        )
        .is_err());
        assert!(plan(
            &base,
            &base.revision,
            &[],
            Change::AddModels {
                binding_name: "example".into(),
                models: vec![model("example_image")],
            }
        )
        .is_err());
        let added = plan(
            &base,
            &base.revision,
            &[],
            Change::AddModels {
                binding_name: "example".into(),
                models: vec![model("example_video")],
            },
        )
        .unwrap();
        assert!(added.binding.is_none());
        assert_eq!(added.changed_offer_ids, ["example_video"]);
    }
}
