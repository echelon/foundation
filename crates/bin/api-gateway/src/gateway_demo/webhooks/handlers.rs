use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use db_queries::{
    gateway_requests::RequestScope,
    gateway_webhooks::{self, Delivery, DeliveryAttempt, NewSubscription, Subscription},
};
use enums::AuditEvent;
use ids::{AccountId, GatewayProjectId, GatewayWebhookDeliveryId, GatewayWebhookSubscriptionId};
use serde::{Deserialize, Serialize};

use super::super::{
    access::parse_id,
    projects::{audit, managed_project, validate_name, OrganizationQuery},
    requests::validate_client_ref,
};
use super::WebhookConfig;
use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

const PAGE_SIZE: usize = 50;

pub fn router(config: Arc<WebhookConfig>) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/projects/:project/webhook-destinations",
            get(destination_ids),
        )
        .route(
            "/v1/projects/:project/webhooks",
            get(list_subscriptions).post(create),
        )
        .route(
            "/v1/projects/:project/webhooks/:subscription",
            delete(revoke),
        )
        .route(
            "/v1/projects/:project/webhooks/:subscription/rotate",
            post(rotate),
        )
        .route(
            "/v1/projects/:project/webhook-deliveries",
            get(list_deliveries),
        )
        .route(
            "/v1/projects/:project/webhook-deliveries/:delivery",
            get(detail),
        )
        .route(
            "/v1/projects/:project/webhook-deliveries/:delivery/redrive",
            post(redrive),
        )
        .layer(Extension(config))
}

struct Project {
    account: AccountId,
    id: GatewayProjectId,
}

impl Project {
    async fn authorized(
        state: &AppState,
        auth: &AuthContext,
        query: &OrganizationQuery,
        raw: &str,
    ) -> ApiResult<Self> {
        let (account, project) = managed_project(state, auth, query, raw).await?;
        Ok(Self {
            account,
            id: project.id,
        })
    }

    fn scope(&self) -> RequestScope<'_> {
        RequestScope {
            account: &self.account,
            project: &self.id,
            client_ref: None,
        }
    }
}

async fn destination_ids(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
    Extension(config): Extension<Arc<WebhookConfig>>,
) -> ApiResult<Json<Vec<String>>> {
    Project::authorized(&state, &auth, &query, &raw).await?;
    Ok(Json(config.destination_ids().map(str::to_owned).collect()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    account_id: String,
    before: Option<String>,
}

#[derive(Serialize)]
struct Page<T> {
    items: Vec<T>,
    next_before: Option<String>,
}

impl<T> Page<T> {
    fn from_rows(mut items: Vec<T>, id: impl Fn(&T) -> &str) -> Self {
        let next_before = (items.len() > PAGE_SIZE).then(|| id(&items[PAGE_SIZE - 1]).to_owned());
        items.truncate(PAGE_SIZE);
        Self { items, next_before }
    }
}

async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<PageQuery>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Page<Subscription>>> {
    let project = Project::authorized(
        &state,
        &auth,
        &OrganizationQuery::new(query.account_id),
        &raw,
    )
    .await?;
    let before = query
        .before
        .as_deref()
        .map(|value| parse_id(value, "before"))
        .transpose()?;
    let subscriptions = gateway_webhooks::subscriptions(
        &state.pool,
        project.scope(),
        before.as_ref(),
        PAGE_SIZE as i64 + 1,
    )
    .await?;
    Ok(Json(Page::from_rows(subscriptions, |row| &row.id)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    name: String,
    destination_id: String,
    client_ref: Option<String>,
}

#[derive(Serialize)]
struct Issued {
    #[serde(flatten)]
    subscription: Subscription,
    signing_secret: String,
}

async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
    Extension(config): Extension<Arc<WebhookConfig>>,
    Json(input): Json<CreateInput>,
) -> ApiResult<(StatusCode, Json<Issued>)> {
    let project = Project::authorized(&state, &auth, &query, &raw).await?;
    let name = validate_name(&input.name)?;
    let client_ref = input.client_ref.as_deref();
    validate_client_ref(client_ref)?;
    let destination = config
        .destination(&input.destination_id)
        .ok_or_else(|| ApiError::BadRequest("unknown webhook destination".into()))?;
    let id = GatewayWebhookSubscriptionId::generate();
    let mut tx = state.pool.begin().await?;
    let subscription = gateway_webhooks::create_subscription(
        &mut *tx,
        project.scope(),
        NewSubscription {
            id: &id,
            name,
            destination_id: &input.destination_id,
            destination_url: destination.as_str(),
            client_ref,
        },
    )
    .await?;
    audit(
        &mut tx,
        &project.account,
        &auth.user.id,
        AuditEvent::GatewayWebhookChanged,
        "gateway_webhook_subscription",
        id.as_str(),
    )
    .await?;
    tx.commit().await?;
    let signing_secret = config.signing_secret(id.as_str(), subscription.signing_version);
    Ok((
        StatusCode::CREATED,
        Json(Issued {
            subscription,
            signing_secret,
        }),
    ))
}

async fn rotate(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, subscription)): Path<(String, String)>,
    Extension(config): Extension<Arc<WebhookConfig>>,
) -> ApiResult<Json<Issued>> {
    let project = Project::authorized(&state, &auth, &query, &raw).await?;
    let id: GatewayWebhookSubscriptionId = parse_id(&subscription, "subscription_id")?;
    let mut tx = state.pool.begin().await?;
    let subscription = gateway_webhooks::rotate_subscription(&mut *tx, project.scope(), &id)
        .await?
        .ok_or_else(|| {
            ApiError::Conflict(
                "subscription is missing, revoked, or already in the rotation window".into(),
            )
        })?;
    audit(
        &mut tx,
        &project.account,
        &auth.user.id,
        AuditEvent::GatewayWebhookChanged,
        "gateway_webhook_subscription",
        id.as_str(),
    )
    .await?;
    tx.commit().await?;
    let signing_secret = config.signing_secret(id.as_str(), subscription.signing_version);
    Ok(Json(Issued {
        subscription,
        signing_secret,
    }))
}

async fn revoke(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, subscription)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let project = Project::authorized(&state, &auth, &query, &raw).await?;
    let id: GatewayWebhookSubscriptionId = parse_id(&subscription, "subscription_id")?;
    let mut tx = state.pool.begin().await?;
    if !gateway_webhooks::revoke_subscription(&mut *tx, project.scope(), &id).await? {
        return Err(ApiError::NotFound("active subscription not found".into()));
    }
    audit(
        &mut tx,
        &project.account,
        &auth.user.id,
        AuditEvent::GatewayWebhookChanged,
        "gateway_webhook_subscription",
        id.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_deliveries(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<PageQuery>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Page<Delivery>>> {
    let project = Project::authorized(
        &state,
        &auth,
        &OrganizationQuery::new(query.account_id),
        &raw,
    )
    .await?;
    let before = query
        .before
        .as_deref()
        .map(|value| parse_id(value, "before"))
        .transpose()?;
    let deliveries = gateway_webhooks::deliveries(
        &state.pool,
        project.scope(),
        None,
        before.as_ref(),
        PAGE_SIZE as i64 + 1,
    )
    .await?;
    Ok(Json(Page::from_rows(deliveries, |row| &row.id)))
}

#[derive(Serialize)]
struct DeliveryDetail {
    delivery: Delivery,
    attempts: Vec<DeliveryAttempt>,
}

async fn detail(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, delivery)): Path<(String, String)>,
) -> ApiResult<Json<DeliveryDetail>> {
    let project = Project::authorized(&state, &auth, &query, &raw).await?;
    let id: GatewayWebhookDeliveryId = parse_id(&delivery, "delivery_id")?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let mut deliveries =
        gateway_webhooks::deliveries(&mut *tx, project.scope(), Some(&id), None, 1).await?;
    let delivery = deliveries
        .pop()
        .ok_or_else(|| ApiError::NotFound("delivery not found".into()))?;
    let attempts = gateway_webhooks::attempts(&mut *tx, project.scope(), &id).await?;
    tx.commit().await?;
    Ok(Json(DeliveryDetail { delivery, attempts }))
}

async fn redrive(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, delivery)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let project = Project::authorized(&state, &auth, &query, &raw).await?;
    let id: GatewayWebhookDeliveryId = parse_id(&delivery, "delivery_id")?;
    let mut tx = state.pool.begin().await?;
    if !gateway_webhooks::redrive(&mut *tx, project.scope(), &id).await? {
        return Err(ApiError::Conflict(
            "redrive requires a failed delivery with an active subscription".into(),
        ));
    }
    audit(
        &mut tx,
        &project.account,
        &auth.user.id,
        AuditEvent::GatewayWebhookChanged,
        "gateway_webhook_delivery",
        id.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
