//! api-gateway — all web ingress (codebase_layout_backend.md §2). Thin binary: config
//! load, tracing init, router mount. Handlers/logic live in the library crates; the split
//! seam between router groups (product / console / admin / webhooks-in) is kept clean.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use db::PgPool;
use serde::Serialize;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    pool: Arc<Option<PgPool>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,api_gateway=debug".into()),
        )
        .init();

    let cfg = config::Config::load()?;

    // DB is best-effort at boot — the server still answers /health if it's down.
    let pool = match db::connect(&cfg.database_url).await {
        Ok(p) => {
            tracing::info!("connected to database");
            Some(p)
        }
        Err(e) => {
            tracing::warn!("database unavailable at boot: {e}");
            None
        }
    };

    let state = AppState {
        pool: Arc::new(pool),
    };

    // SPLIT-SEAM: product router group (public + authed API). Other groups (console,
    // admin, webhooks-in) mount here later as separate modules.
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/models/:slug", get(get_model))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.api_gateway_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("api-gateway listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    database: &'static str,
    models: usize,
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    let database = match state.pool.as_ref() {
        Some(pool) if db::ping(pool).await => "up",
        Some(_) => "down",
        None => "unconfigured",
    };
    Json(Health {
        status: "ok",
        database,
        models: catalog::MODELS.len(),
    })
}

/// Public catalog read — `GET /v1/models` (model_rankings_and_reports.md / api_misc.md).
async fn list_models() -> Json<&'static [catalog::ModelSpec]> {
    Json(catalog::MODELS)
}

async fn get_model(Path(slug): Path<String>) -> Result<Json<&'static catalog::ModelSpec>, StatusCode> {
    catalog::by_slug(&slug)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
