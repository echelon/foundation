use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use enums::MemberRole;
use serde::Serialize;
use utoipa::ToSchema;

use super::{process::ProcessRole, projects::OrganizationQuery};
use crate::{
    authz::require_account_role, error::ApiResult, middleware::AuthContext, state::AppState,
};

const CHECK_INTERVAL: Duration = Duration::from_secs(5);
const EXPIRE_AFTER: Duration = Duration::from_secs(15);
const DB_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Checking,
    Ready,
    Unavailable,
    NotConfigured,
    External,
}

impl From<bool> for CheckStatus {
    fn from(ready: bool) -> Self {
        if ready {
            Self::Ready
        } else {
            Self::Unavailable
        }
    }
}

struct Observation(Mutex<(Instant, CheckStatus)>);

impl Observation {
    fn new() -> Self {
        Self(Mutex::new((Instant::now(), CheckStatus::Checking)))
    }

    fn record(&self, ready: bool) {
        if let Ok(mut state) = self.0.lock() {
            *state = (Instant::now(), ready.into());
        }
    }

    fn status(&self) -> CheckStatus {
        match self.0.lock().ok().map(|state| *state) {
            Some((observed, status)) if observed.elapsed() < EXPIRE_AFTER => status,
            _ => CheckStatus::Unavailable,
        }
    }
}

pub(super) struct Readiness {
    role: ProcessRole,
    database: Observation,
    schema: Observation,
    queue: Observation,
    provider: Option<Observation>,
    artifact: Option<Observation>,
    artifact_backend: &'static str,
}

impl Readiness {
    pub(super) fn new(role: ProcessRole, provider_enabled: bool, artifact_backend: &'static str) -> Self {
        Self {
            role,
            database: Observation::new(),
            schema: Observation::new(),
            queue: Observation::new(),
            provider: provider_enabled.then(Observation::new),
            artifact: (artifact_backend != "postgresql").then(Observation::new),
            artifact_backend,
        }
    }

    pub(super) fn queue_observed(&self, success: bool) {
        self.queue.record(success);
    }

    pub(super) fn provider_observed(&self, success: bool) {
        if let Some(provider) = &self.provider {
            provider.record(success);
        }
    }

    pub(super) fn artifact_observed(&self, success: bool) {
        if let Some(artifact) = &self.artifact {
            artifact.record(success);
        }
    }

    pub(super) fn observe_database(self: Arc<Self>, pool: db::PgPool) {
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(CHECK_INTERVAL);
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                timer.tick().await;
                let conn = tokio::time::timeout(DB_TIMEOUT, async {
                    let mut conn = pool.acquire().await?;
                    sqlx::query("SELECT 1").execute(&mut *conn).await?;
                    Ok::<_, sqlx::Error>(conn)
                })
                .await;
                let Ok(Ok(mut conn)) = conn else {
                    self.database.record(false);
                    self.schema.record(false);
                    continue;
                };
                self.database.record(true);
                let current =
                    tokio::time::timeout(DB_TIMEOUT, db::migrations_current(&mut conn)).await;
                self.schema.record(matches!(current, Ok(Ok(true))));
            }
        });
    }

    fn worker_status(&self, worker: &Observation) -> CheckStatus {
        if self.role.runs_workers() {
            worker.status()
        } else {
            CheckStatus::External
        }
    }

    fn snapshot(&self) -> SetupChecks {
        let database = self.database.status();
        let migrations = self.schema.status();
        let worker = self.worker_status(&self.queue);
        let provider_worker = self
            .provider
            .as_ref()
            .map_or(CheckStatus::NotConfigured, |provider| {
                self.worker_status(provider)
            });
        let artifact_storage = self.artifact.as_ref().map_or(migrations, Observation::status);
        let ready = [
            database,
            migrations,
            worker,
            provider_worker,
            artifact_storage,
        ]
        .iter()
        .all(|status| {
            matches!(
                status,
                CheckStatus::Ready | CheckStatus::NotConfigured | CheckStatus::External
            )
        });
        SetupChecks {
            ready,
            database,
            migrations,
            worker,
            provider_worker,
            artifact_storage,
            artifact_backend: self.artifact_backend,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub(super) struct ReadyResponse {
    ready: bool,
}

#[derive(Serialize, ToSchema)]
pub(super) struct SetupChecks {
    ready: bool,
    database: CheckStatus,
    migrations: CheckStatus,
    worker: CheckStatus,
    provider_worker: CheckStatus,
    artifact_storage: CheckStatus,
    artifact_backend: &'static str,
}

#[utoipa::path(
    get, path = "/ready", tag = "meta",
    responses(
        (status = 200, description = "The scoped gateway is ready.", body = ReadyResponse),
        (status = 503, description = "One or more gateway readiness checks have not passed.", body = ReadyResponse),
    ),
)]
pub(super) async fn ready(
    Extension(readiness): Extension<Arc<Readiness>>,
) -> (StatusCode, Json<ReadyResponse>) {
    let ready = readiness.snapshot().ready;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(ReadyResponse { ready }))
}

#[utoipa::path(
    get, path = "/v1/setup/status", tag = "meta", security(("session_cookie" = [])),
    params(("account_id" = String, Query, description = "Organization whose administrator is requesting setup status.")),
    responses((status = 200, description = "Recent setup checks for this gateway instance.", body = SetupChecks)),
)]
pub(super) async fn setup(
    State(state): State<AppState>,
    Extension(readiness): Extension<Arc<Readiness>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
) -> ApiResult<Json<SetupChecks>> {
    let account = query.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    Ok(Json(readiness.snapshot()))
}
