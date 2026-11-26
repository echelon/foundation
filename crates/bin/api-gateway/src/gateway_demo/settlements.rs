use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use db_queries::gateway_requests::RequestScope;
use db_queries::gateway_supplier_settlements::{self as supplier, NewPosting, Posting};
use enums::AuditEvent;
use ids::GatewayRequestId;
use serde::{Deserialize, Serialize};

use super::catalog::{digest, valid_identifier};
use super::handlers::parse_request_id;
use super::projects::{audit, managed_project, OrganizationQuery};
use super::requests::valid_idempotency_key;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Serialize)]
pub(super) struct History {
    request_id: GatewayRequestId,
    eligible: bool,
    provider: String,
    estimated_usd_nanos: String,
    entries: Vec<Settlement>,
}

#[derive(Serialize)]
pub(super) struct Settlement {
    revision: i32,
    amount_usd_nanos: String,
    delta_usd_nanos: String,
    evidence_ref: String,
    recorded_at: DateTime<Utc>,
}

impl From<Posting> for Settlement {
    fn from(posting: Posting) -> Self {
        Self {
            revision: posting.revision,
            amount_usd_nanos: posting.amount_usd_nanos.to_string(),
            delta_usd_nanos: posting.delta_usd_nanos.to_string(),
            evidence_ref: posting.evidence_ref,
            recorded_at: posting.recorded_at,
        }
    }
}

pub(super) async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((project, raw)): Path<(String, String)>,
) -> ApiResult<Json<History>> {
    let (account, project) = managed_project(&state, &auth, &query, &project).await?;
    let request_id = parse_request_id(&raw)?;
    let scope = RequestScope {
        account: &account,
        project: &project.id,
        client_ref: None,
    };
    let mut tx = state.pool.begin().await?;
    let target = supplier::target(&mut *tx, scope, &request_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("request not found in this project".into()))?;
    let entries = supplier::history(&mut *tx, &request_id)
        .await?
        .into_iter()
        .map(Settlement::from)
        .collect();
    tx.commit().await?;
    Ok(Json(History {
        request_id,
        eligible: target.eligible,
        provider: target.provider,
        estimated_usd_nanos: target.estimated_usd_nanos.to_string(),
        entries,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordInput {
    expected_revision: i32,
    amount_usd_nanos: String,
    evidence_ref: String,
}

pub(super) async fn record(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((project, raw)): Path<(String, String)>,
    headers: HeaderMap,
    Json(input): Json<RecordInput>,
) -> ApiResult<(StatusCode, Json<Settlement>)> {
    let (account, project) = managed_project(&state, &auth, &query, &project).await?;
    let request = parse_request_id(&raw)?;
    let scope = RequestScope {
        account: &account,
        project: &project.id,
        client_ref: None,
    };
    let mut keys = headers.get_all("idempotency-key").iter();
    let key = keys
        .next()
        .and_then(|value| value.to_str().ok())
        .filter(|value| keys.next().is_none() && valid_idempotency_key(value))
        .ok_or_else(|| ApiError::BadRequest("supply exactly one valid Idempotency-Key".into()))?;
    let key = digest(key);
    let amount = input
        .amount_usd_nanos
        .parse::<i64>()
        .ok()
        .filter(|_| {
            input
                .amount_usd_nanos
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| {
            ApiError::BadRequest("use a decimal string of nonnegative integer nano-USD".into())
        })?;
    let revision = input
        .expected_revision
        .checked_add(1)
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::BadRequest("invalid expected settlement revision".into()))?;
    if !valid_identifier(&input.evidence_ref) {
        return Err(ApiError::BadRequest(
            "use an opaque evidence reference up to 64 ASCII characters".into(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    let target = supplier::target(&mut *tx, scope, &request)
        .await?
        .ok_or_else(|| ApiError::NotFound("request not found in this project".into()))?;
    if let Some(posting) = supplier::replay(&mut *tx, &request, &key).await? {
        if (
            posting.revision,
            posting.amount_usd_nanos,
            posting.evidence_ref.as_str(),
        ) != (revision, amount, input.evidence_ref.as_str())
        {
            return Err(ApiError::Conflict(
                "this key already recorded a different settlement".into(),
            ));
        }
        return Ok((StatusCode::OK, Json(posting.into())));
    }
    if !target.eligible {
        return Err(ApiError::Conflict(
            "only dispatched provider requests with a recorded outcome can be settled".into(),
        ));
    }
    let latest = supplier::latest_revision(&mut *tx, &request)
        .await?
        .unwrap_or(0);
    if latest != input.expected_revision {
        return Err(ApiError::Conflict(
            "the settlement changed; reload before posting a correction".into(),
        ));
    }
    let posting = supplier::append(
        &mut *tx,
        NewPosting {
            request: &request,
            revision,
            amount_usd_nanos: amount,
            evidence_ref: &input.evidence_ref,
            idempotency_digest: &key,
        },
    )
    .await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::GatewaySupplierSettlementRecorded,
        "gateway_supplier_settlement",
        &format!("{request}/{revision}"),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(posting.into())))
}
