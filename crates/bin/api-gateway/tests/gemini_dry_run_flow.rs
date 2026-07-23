//! End-to-end DRY-RUN coverage for every Gemini model — Nano Banana (all four
//! variants), Veo (all three tiers), and Omni — on BOTH API planes (the
//! model-specific `/v1/models/{slug}/generate` and the Any plane) plus the cost
//! twins. Dry runs exercise the full enqueue path (validate → route → price →
//! persist → outbound request LOGGING) without spending a cent: the job lands
//! terminal (`canceled` / `dry_run`) and every would-be provider call leaves a
//! pre-send `provider_calls` row whose id is the request_id — exactly the
//! OpenAI logging contract.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;

const NANO_BANANA_SLUGS: [&str; 4] =
  ["google_nano_banana", "google_nano_banana_pro", "google_nano_banana_2", "google_nano_banana_2_lite"];
const VIDEO_SLUGS: [&str; 4] =
  ["google_veo_3_1", "google_veo_3_1_fast", "google_veo_3_1_lite", "google_omni_flash"];

async fn app_and_pool() -> (Router, db::PgPool) {
  let pool = db::test_fixtures::create_test_pool().await;
  let state = AppState {
    pool: pool.clone(),
    jwt: Arc::new(JwtKeys::new("test-secret")),
    cookie_domain: "localhost".into(),
    cookie_secure: false,
    app_base_url: "http://localhost:3000".into(),
    email_from: "Foundation <noreply@localhost>".into(),
    resend_api_key: None,
    bucket: None,
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: None,
    google: None,
    github: None,
    openai: None,
    openai_webhook_secret: None,
  };
  (api_gateway::build_router(state), pool)
}

fn req(method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
  let mut b = Request::builder().method(method).uri(uri);
  if body.is_some() {
    b = b.header("content-type", "application/json");
  }
  if let Some(c) = cookie {
    b = b.header(header::COOKIE, c);
  }
  match body {
    Some(v) => b.body(Body::from(serde_json::to_vec(&v).unwrap())).unwrap(),
    None => b.body(Body::empty()).unwrap(),
  }
}

fn session_cookie(resp: &axum::response::Response) -> Option<String> {
  for v in resp.headers().get_all(header::SET_COOKIE) {
    let s = v.to_str().ok()?;
    if let Some(rest) = s.strip_prefix("foundation_session=") {
      let token = rest.split(';').next().unwrap_or("");
      if !token.is_empty() {
        return Some(format!("foundation_session={token}"));
      }
    }
  }
  None
}

async fn body_json(resp: axum::response::Response) -> Value {
  let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
  serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn signup_owner(app: &Router, pool: &db::PgPool) -> (String, ids::AccountId) {
  let email = format!("user-{}@example.com", id_core::mint("t"));
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/signup", Some(json!({ "email": email, "password": "password1" })), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).unwrap();
  let user_id: ids::UserId = body_json(resp).await["user"]["id"].as_str().unwrap().parse().unwrap();
  let account_id = db_queries::members::list_memberships_for_user(pool, &user_id).await.unwrap()[0]
    .account_id
    .clone();
  (cookie, account_id)
}

/// The full dry-run contract, asserted uniformly for every model/plane:
/// 201, terminal `canceled`/`dry_run`, nothing at the provider, and one
/// pre-send `provider_calls` row PER would-be call carrying the exact payload.
async fn assert_dry_run_job(
  pool: &db::PgPool,
  account_id: &ids::AccountId,
  job: &Value,
  slug: &str,
  expected_calls: usize,
  expected_url_fragment: &str,
  expected_price_unit: &str,
) {
  assert_eq!(job["model"], slug);
  assert_eq!(job["dry_run"], true);
  assert_eq!(job["status"], "canceled");
  assert_eq!(job["failure_code"], "dry_run");
  assert_eq!(job["outputs"].as_array().unwrap().len(), 0);
  assert!(job["estimated_micros"].as_i64().unwrap() > 0, "{slug}: dry runs still price");

  let job_id: ids::JobId = job["id"].as_str().unwrap().parse().unwrap();
  let db_job = db_queries::jobs::find_job_for_account(pool, account_id, &job_id).await.unwrap().unwrap();
  assert_eq!(db_job.provider.as_deref(), Some(catalog::Provider::Google.as_str()), "{slug}");
  assert_eq!(db_job.status, enums::JobStatus::Canceled, "{slug}");
  assert!(db_job.provider_job_id.is_none(), "{slug}: nothing was sent");
  let spec = catalog::by_slug(slug).unwrap();
  assert_eq!(db_job.model, spec.slug);
  let stored_unit = db_queries::jobs::get_job_price_unit(pool, &job_id).await.unwrap().unwrap();
  assert_eq!(stored_unit, expected_price_unit, "{slug}");

  // The vendor request log: one pre-send row per would-be call, id = request_id,
  // exact payload captured, flagged dry-run — identical to the OpenAI contract.
  let calls = db_queries::provider_calls::list_provider_calls_for_job(pool, &job_id).await.unwrap();
  assert_eq!(calls.len(), expected_calls, "{slug}: one provider_calls row per would-be call");
  for call in &calls {
    assert!(call.id.as_str().starts_with("pcall_"), "{slug}: minted request_id");
    assert_eq!(call.provider, catalog::Provider::Google.as_str());
    assert!(call.is_dry_run);
    assert!(call.provider_job_id.is_none());
    assert!(
      call.url.as_deref().unwrap_or_default().contains(expected_url_fragment),
      "{slug}: url {:?} should contain {expected_url_fragment}",
      call.url
    );
    let body = call.request_body.as_ref().expect("exact payload logged");
    assert!(body.get("contents").is_some() || body.get("instances").is_some() || body.get("input").is_some());
  }
  // Fanned dry runs cancel every slice too.
  if expected_calls > 1 {
    assert_eq!(job["progress"]["total"], expected_calls as i64);
    assert_eq!(job["progress"]["completed"], expected_calls as i64);
    let slices = db_queries::job_slices::list_slices_for_job(pool, &job_id).await.unwrap();
    assert_eq!(slices.len(), expected_calls);
    assert!(slices.iter().all(|s| s.status == enums::JobSliceStatus::Canceled), "{slug}");
  }
}

#[tokio::test]
async fn nano_banana_dry_runs_on_both_planes_with_logged_payloads() {
  let (app, pool) = app_and_pool().await;
  let (cookie, account_id) = signup_owner(&app, &pool).await;

  for slug in NANO_BANANA_SLUGS {
    // ── model-specific plane, fanned (n=2) ──
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        &format!("/v1/models/{slug}/generate"),
        Some(json!({
          "account_id": account_id,
          "prompt": "A watercolor lighthouse at dusk",
          "n": 2,
          "quality": "high",
          "dry_run": true
        })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "{slug}");
    let job = body_json(resp).await;
    assert_eq!(job["api_shape"], "unified", "{slug}");
    assert_dry_run_job(&pool, &account_id, &job, slug, 2, ":generateContent", "per_image").await;

    // ── Any plane, single ──
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        "/v1/any/image",
        Some(json!({
          "model": slug,
          "account_id": account_id,
          "prompt": "A watercolor lighthouse at dusk",
          "dry_run": true
        })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "{slug}");
    let job = body_json(resp).await;
    assert_eq!(job["api_shape"], "any", "{slug}");
    assert_dry_run_job(&pool, &account_id, &job, slug, 1, ":generateContent", "per_image").await;
  }
}

#[tokio::test]
async fn video_models_dry_run_on_both_planes_with_logged_payloads() {
  let (app, pool) = app_and_pool().await;
  let (cookie, account_id) = signup_owner(&app, &pool).await;

  for slug in VIDEO_SLUGS {
    let is_omni = slug == foundation_router::video::plan::RouterVideoModel::GoogleOmniFlash.slug();
    let url_fragment = if is_omni { "/interactions" } else { ":predictLongRunning" };

    // ── model-specific plane, fanned (n=2), superset params (prune/coerce paths) ──
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        &format!("/v1/models/{slug}/generate"),
        Some(json!({
          "account_id": account_id,
          "prompt": "a paper boat drifts down a rainy gutter",
          "negative_prompt": "text overlays",
          "n": 2,
          "duration_seconds": 4,
          "aspect_ratio": "9:16",
          "dry_run": true
        })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "{slug}");
    let job = body_json(resp).await;
    assert_eq!(job["api_shape"], "unified", "{slug}");
    assert_dry_run_job(&pool, &account_id, &job, slug, 2, url_fragment, "per_video_second").await;
    if is_omni {
      // Omni prunes duration + folds the negative prompt — both reported.
      let params: Vec<&str> =
        job["param_adjustments"].as_array().unwrap().iter().map(|a| a["param"].as_str().unwrap()).collect();
      assert!(params.contains(&"duration_seconds"), "omni prunes duration");
      assert!(params.contains(&"negative_prompt"), "omni folds the negative prompt");
    }

    // ── Any plane, single ──
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        "/v1/any/video",
        Some(json!({
          "model": slug,
          "account_id": account_id,
          "prompt": "a paper boat drifts down a rainy gutter",
          "dry_run": true
        })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "{slug}");
    let job = body_json(resp).await;
    assert_eq!(job["api_shape"], "any", "{slug}");
    assert_dry_run_job(&pool, &account_id, &job, slug, 1, url_fragment, "per_video_second").await;
  }
}

#[tokio::test]
async fn cost_twins_quote_every_gemini_model_on_both_planes() {
  let (app, pool) = app_and_pool().await;
  let (cookie, account_id) = signup_owner(&app, &pool).await;

  // Certainty mirrors each client calculator: the original Nano Banana is a flat
  // per-image price (exact); the gen-2/Pro tiers bill thinking tokens on top of
  // the tier, so their quotes are estimates by design.
  for (slug, certainty) in [
    ("google_nano_banana", "exact"),
    ("google_nano_banana_pro", "estimated"),
    ("google_nano_banana_2", "estimated"),
    ("google_nano_banana_2_lite", "estimated"),
  ] {
    for (uri, body) in [
      (
        format!("/v1/models/{slug}/generate/cost"),
        json!({ "account_id": account_id, "prompt": "p", "n": 3, "quality": "medium" }),
      ),
      ("/v1/any/image/cost".to_string(), json!({ "model": slug, "account_id": account_id, "prompt": "p", "n": 3 })),
    ] {
      let resp = app.clone().oneshot(req("POST", &uri, Some(body), Some(&cookie))).await.unwrap();
      assert_eq!(resp.status(), StatusCode::OK, "{slug} {uri}");
      let quote = body_json(resp).await;
      assert!(quote["amount_micros"].as_i64().unwrap() > 0, "{slug} {uri}");
      assert_eq!(quote["line_items"].as_array().unwrap().len(), 3, "{slug} {uri}: one line per image");
      assert_eq!(quote["certainty"], certainty, "{slug} {uri}");
    }
  }

  for slug in VIDEO_SLUGS {
    let is_omni = slug == foundation_router::video::plan::RouterVideoModel::GoogleOmniFlash.slug();
    for (uri, body) in [
      (
        format!("/v1/models/{slug}/generate/cost"),
        json!({ "account_id": account_id, "prompt": "p", "n": 2, "duration_seconds": 4 }),
      ),
      ("/v1/any/video/cost".to_string(), json!({ "model": slug, "account_id": account_id, "prompt": "p", "n": 2 })),
    ] {
      let resp = app.clone().oneshot(req("POST", &uri, Some(body), Some(&cookie))).await.unwrap();
      assert_eq!(resp.status(), StatusCode::OK, "{slug} {uri}");
      let quote = body_json(resp).await;
      assert!(quote["amount_micros"].as_i64().unwrap() > 0, "{slug} {uri}");
      assert_eq!(quote["line_items"].as_array().unwrap().len(), 2, "{slug} {uri}: one line per video");
      // Veo prices exactly; Omni is token-billed → estimated by nature.
      assert_eq!(quote["certainty"], if is_omni { "estimated" } else { "exact" }, "{slug} {uri}");
    }
  }
}

#[tokio::test]
async fn dry_run_requests_replay_through_the_typed_request_endpoint() {
  let (app, pool) = app_and_pool().await;
  let (cookie, account_id) = signup_owner(&app, &pool).await;

  // A video dry run's originating request comes back in the video wire shape…
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/models/google_veo_3_1_fast/generate",
      Some(json!({
        "account_id": account_id,
        "prompt": "a candle flame",
        "negative_prompt": "smoke",
        "n": 2,
        "resolution": "1080p",
        "duration_seconds": 4,
        "aspect_ratio": "16:9",
        "dry_run": true
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job_id = body_json(resp).await["id"].as_str().unwrap().to_string();
  let resp = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}/request?account_id={account_id}"), None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let request = body_json(resp).await;
  assert_eq!(request["media_type"], catalog::MediaType::Video.as_str());
  assert_eq!(request["prompt"], "a candle flame");
  assert_eq!(request["negative_prompt"], "smoke");
  assert_eq!(request["resolution"], "1080p");
  // The stored request keeps the CALLER'S duration — replaying it re-applies the
  // 1080p→8s coercion (which the job reports in param_adjustments).
  assert_eq!(request["duration_seconds"], 4);
  assert_eq!(request["aspect_ratio"], "16:9");
  assert_eq!(request["dry_run"], true);
  assert!(request.get("size").is_none(), "image params are omitted on video jobs");

  // …and an image dry run still comes back in the image shape.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/models/google_nano_banana_2/generate",
      Some(json!({ "account_id": account_id, "prompt": "a red fox", "dry_run": true })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job_id = body_json(resp).await["id"].as_str().unwrap().to_string();
  let resp = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}/request?account_id={account_id}"), None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let request = body_json(resp).await;
  assert_eq!(request["media_type"], catalog::MediaType::Image.as_str());
  assert_eq!(request["prompt"], "a red fox");
  let _ = pool; // pool is used by signup_owner; keep the binding obvious
}
