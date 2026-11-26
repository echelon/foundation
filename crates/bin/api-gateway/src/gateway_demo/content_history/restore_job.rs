use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use db_queries::gateway_requests::content_restores::{self as jobs, Update};
use enums::AuditEvent;
use serde::{Deserialize, Serialize};

use super::super::{
    access::{self, GatewayCaller, Permission, ScopeQuery},
    archive_blobs::{COLD_OBJECT, IO_ERROR},
    catalog::digest,
    handlers::parse_request_id,
    projects::audit,
    providers::ProviderRuntime,
    RequestErrorResponse,
};
use super::{artifact_name, storage::Target, Store, MAX_BUNDLE};
use crate::{
    error::{ApiError, ApiResult},
    middleware::Caller,
    state::AppState,
};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum RestoreState {
    Pending,
    Ready,
    Failed,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RestoreProgress {
    state: RestoreState,
    ready_objects: i32,
    total_objects: i32,
    requested_at: DateTime<Utc>,
    retry_at: DateTime<Utc>,
}

impl From<jobs::Progress> for RestoreProgress {
    fn from(job: jobs::Progress) -> Self {
        Self {
            state: match job.state.as_str() {
                "pending" => RestoreState::Pending,
                "ready" => RestoreState::Ready,
                _ => RestoreState::Failed,
            },
            ready_objects: job.ready_objects,
            total_objects: job.total_objects,
            requested_at: job.requested_at,
            retry_at: job.retry_at,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestoreInput {}

#[utoipa::path(
    post, path = "/v1/requests/{id}/content/restore", operation_id = "queueRequestContentRestore", tag = "native",
    summary = "Queue a storage restore as a project administrator",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Request ID"), ScopeQuery),
    request_body = RestoreInput,
    responses((status = 202, body = RestoreProgress), (status = "default", response = RequestErrorResponse)),
)]
pub(crate) async fn enqueue(
    State(state): State<AppState>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    caller: GatewayCaller,
    Query(scope): Query<ScopeQuery>,
    Path(raw): Path<String>,
    Json(_input): Json<RestoreInput>,
) -> ApiResult<(StatusCode, Json<RestoreProgress>)> {
    let GatewayCaller::Account(Caller::Session(auth)) = &caller else {
        return Err(ApiError::Forbidden("archive restore requires an administrator session".into()));
    };
    let actor = auth.user.id.clone();
    let access = access::resolve(&state, caller, scope, Permission::ReadRequests).await?;
    if !access.can_restore_content {
        return Err(ApiError::Forbidden("archive restore requires a project administrator".into()));
    }
    let scope = access.scope();
    let id = parse_request_id(&raw)?;
    let missing = || ApiError::NotFound("archived request content is unavailable".into());
    let mut tx = state.pool.begin().await?;
    let target = jobs::lock_archive(&mut tx, scope, &id).await?.ok_or_else(missing)?;
    if !runtime.as_ref().and_then(|runtime| runtime.content_archive.as_ref())
        .is_some_and(|store| store.permits_restore(&target))
    {
        return Err(ApiError::Conflict("gateway restore is disabled for this archive".into()));
    }
    if jobs::enqueue(&mut tx, &id, MAX_BUNDLE as i32).await? {
        audit(&mut tx, scope.account, &actor, AuditEvent::GatewayRequestContentRestoreQueued,
              "gateway_request", id.as_str()).await?;
    }
    let progress = jobs::progress(&mut *tx, &id).await?.ok_or_else(missing)?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(progress.into())))
}

pub(super) fn spawn(pool: db::PgPool, store: Arc<Store>) {
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Err(error) = work(&pool, &store.primary).await {
                tracing::warn!(sqlstate = ?error.as_database_error().and_then(|e| e.code()),
                    "request content restore deferred");
            }
        }
    });
}

async fn work(pool: &db::PgPool, target: &Target) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let Some(task) = jobs::claim(&mut tx, target.id()).await? else {
        return Ok(());
    };
    let Ok(id) = task.request_id.parse() else {
        jobs::update(&mut tx, &task, Update::Failed).await?;
        return tx.commit().await;
    };
    let name = if task.slot == -1 {
        format!("{id}.tch")
    } else {
        artifact_name(&id, task.slot)
    };
    let update = match target.read(&name, task.byte_limit as usize).await {
        Ok(ciphertext) if digest(&ciphertext) == task.checksum => Update::Ready,
        Err(COLD_OBJECT) if !task.attempted => Update::Attempt,
        Err(COLD_OBJECT | IO_ERROR) => Update::Wait,
        _ => Update::Failed,
    };
    let dispatch = matches!(update, Update::Attempt);
    jobs::update(&mut tx, &task, update).await?;
    tx.commit().await?;
    if dispatch {
        let mut tx = pool.begin().await?;
        if jobs::can_dispatch(&mut tx, target.id(), &task).await?
            && target.request_restore(&name).await.is_err()
        {
            jobs::update(&mut tx, &task, Update::Failed).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}
