//! End-to-end generation endpoints: enqueue on the Any and unified planes (fan-out slice
//! rows, partial-completion progress shape), cost twins with USD-micros quotes and param
//! adjustments, job polling + listing, tenancy isolation, and the staff-only admin list.
//!
//! No dispatcher runs here (build_router only), so jobs stay `queued` — exactly what the
//! enqueue contract promises before a provider accepts.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;

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

fn unique_email() -> String {
  format!("user-{}@example.com", id_core::mint("t"))
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

async fn signup_owner(app: &Router, pool: &db::PgPool) -> (String, ids::UserId, ids::AccountId) {
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/signup", Some(json!({ "email": unique_email(), "password": "password1" })), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).unwrap();
  let user_id: ids::UserId = body_json(resp).await["user"]["id"].as_str().unwrap().parse().unwrap();
  let account_id = db_queries::members::list_memberships_for_user(pool, &user_id).await.unwrap()[0]
    .account_id
    .clone();
  (cookie, user_id, account_id)
}

#[tokio::test]
async fn any_plane_enqueues_a_fanned_out_job_with_progress() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id, account_id) = signup_owner(&app, &pool).await;

  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({
        "model": "openai_gpt_image_1_5",
        "account_id": account_id,
        "prompt": "A watercolor lighthouse at dusk",
        "n": 3,
        "size": "1024x1536",
        "quality": "high",
        "output_format": "webp"
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job = body_json(resp).await;
  let job_id = job["id"].as_str().unwrap().to_string();
  assert!(job_id.starts_with("job_"));
  assert_eq!(job["model"], "openai_gpt_image_1_5");
  assert_eq!(job["status"], "queued");
  // n=3 fans out: slice rows exist and progress reports 0/3 done.
  assert_eq!(job["progress"]["total"], 3);
  assert_eq!(job["progress"]["completed"], 0);
  assert_eq!(job["outputs"].as_array().unwrap().len(), 0);
  assert!(job["estimated_micros"].as_i64().unwrap() > 0);

  // The row landed with the right owner/source/shape.
  let db_job = db_queries::jobs::find_job_for_account(&pool, &account_id, &job_id.parse().unwrap())
    .await
    .unwrap()
    .unwrap();
  assert_eq!(db_job.user_id.as_ref(), Some(&user_id));
  assert_eq!(db_job.source, enums::JobSource::Dashboard);
  assert_eq!(db_job.api_shape, enums::ApiShape::Any);
  assert_eq!(db_job.modality, "text_to_image");
  let slices = db_queries::job_slices::list_slices_for_job(&pool, &db_job.id).await.unwrap();
  assert_eq!(slices.len(), 3);
  assert!(slices.iter().all(|s| s.status == enums::JobSliceStatus::Queued));

  // Lightweight status poll agrees.
  let status = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}/status?account_id={account_id}"), None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(status.status(), StatusCode::OK);
  let status = body_json(status).await;
  assert_eq!(status["status"], "queued");
  assert_eq!(status["progress"]["total"], 3);

  // Full fetch and the account listing both return the job.
  let full = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}?account_id={account_id}"), None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(full.status(), StatusCode::OK);
  let list = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs?account_id={account_id}"), None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(list.status(), StatusCode::OK);
  let list = body_json(list).await;
  assert_eq!(list["jobs"].as_array().unwrap().len(), 1);
  assert_eq!(list["jobs"][0]["id"], job_id.as_str());
  assert!(list["next_after"].is_null());
}

#[tokio::test]
async fn unified_plane_single_image_job_has_no_slices() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;

  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/models/openai_gpt_image_1_mini/generate",
      Some(json!({ "account_id": account_id, "prompt": "a fox" })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job = body_json(resp).await;
  assert_eq!(job["status"], "queued");
  // n=1: NO fan-out — no progress block, no slice rows.
  assert!(job["progress"].is_null());
  let db_job = db_queries::jobs::find_job_for_account(
    &pool,
    &account_id,
    &job["id"].as_str().unwrap().parse().unwrap(),
  )
  .await
  .unwrap()
  .unwrap();
  assert_eq!(db_job.api_shape, enums::ApiShape::Unified);
  assert!(db_queries::job_slices::list_slices_for_job(&pool, &db_job.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn cost_twins_quote_micros_and_record_adjustments() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;

  // gpt-image-2 high, n=2 — and input_fidelity, which gpt-image-2 doesn't support: the
  // quote must arrive with the pruning recorded, not a rejection.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image/cost",
      Some(json!({
        "model": "openai_gpt_image_2",
        "account_id": account_id,
        "prompt": "neon koi pond",
        "image_inputs": ["https://cdn.example.com/source.png"],
        "n": 2,
        "quality": "high",
        "input_fidelity": "high"
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let quote = body_json(resp).await;
  assert_eq!(quote["certainty"], "estimated");
  // gpt-image-2 high = 211_000 micros per slice (client CostCalculator), 2 slices.
  assert_eq!(quote["amount_micros"], 422_000);
  assert_eq!(quote["line_items"].as_array().unwrap().len(), 2);
  assert!(quote["display"].as_str().unwrap().starts_with('$'));
  let adjustments = quote["param_adjustments"].as_array().unwrap();
  assert!(
    adjustments.iter().any(|a| a["param"] == "input_fidelity" && a["action"] == "pruned"),
    "expected input_fidelity pruning to be recorded: {adjustments:?}"
  );

  // The unified twin quotes the same request identically.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/models/openai_gpt_image_2/generate/cost",
      Some(json!({
        "account_id": account_id,
        "prompt": "neon koi pond",
        "image_inputs": ["https://cdn.example.com/source.png"],
        "n": 2,
        "quality": "high",
        "input_fidelity": "high"
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  assert_eq!(body_json(resp).await["amount_micros"], 422_000);
}

#[tokio::test]
async fn invalid_requests_are_rejected() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;

  // Unknown model: Any plane 400 (body param), unified plane 404 (path resource).
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "nope_9", "account_id": account_id, "prompt": "x" })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/models/nope_9/generate",
      Some(json!({ "account_id": account_id, "prompt": "x" })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  // Empty prompt and non-https image inputs are 400s.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "openai_gpt_image_1", "account_id": account_id, "prompt": "   " })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({
        "model": "openai_gpt_image_1",
        "account_id": account_id,
        "prompt": "x",
        "image_inputs": ["http://plain.example.com/a.png"]
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

  // n over the cap is CLAMPED to 10 (coercion, not rejection — Any-plane semantics).
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "openai_gpt_image_1_mini", "account_id": account_id, "prompt": "x", "n": 200 })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job = body_json(resp).await;
  assert_eq!(job["progress"]["total"], 10);
  let adjustments = job["param_adjustments"].as_array().unwrap();
  assert!(adjustments.iter().any(|a| a["param"] == "n" && a["action"] == "coerced"));
}

#[tokio::test]
async fn jobs_are_tenancy_isolated() {
  let (app, pool) = app_and_pool().await;
  let (cookie_a, _ua, account_a) = signup_owner(&app, &pool).await;
  let (cookie_b, _ub, account_b) = signup_owner(&app, &pool).await;

  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "openai_gpt_image_1", "account_id": account_a, "prompt": "private" })),
      Some(&cookie_a),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job_id = body_json(resp).await["id"].as_str().unwrap().to_string();

  // B can't enqueue INTO A's account…
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "openai_gpt_image_1", "account_id": account_a, "prompt": "sneaky" })),
      Some(&cookie_b),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  // …can't read A's job through their own account scope…
  let resp = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}?account_id={account_b}"), None, Some(&cookie_b)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  // …and can't borrow A's account scope either.
  let resp = app
    .clone()
    .oneshot(req("GET", &format!("/v1/jobs/{job_id}?account_id={account_a}"), None, Some(&cookie_b)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_jobs_list_is_staff_only_and_paginates() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id, account_id) = signup_owner(&app, &pool).await;

  let resp =
    app.clone().oneshot(req("GET", "/v1/admin/jobs", None, Some(&cookie))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::FORBIDDEN);

  sqlx::query("UPDATE users SET is_staff = true WHERE id = $1")
    .bind(user_id.as_str())
    .execute(&pool)
    .await
    .unwrap();

  for i in 0..3 {
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        "/v1/any/image",
        Some(json!({ "model": "openai_gpt_image_1", "account_id": account_id, "prompt": format!("job {i}") })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
  }

  let resp = app.clone().oneshot(req("GET", "/v1/admin/jobs", None, Some(&cookie))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let page = body_json(resp).await;
  let jobs = page["jobs"].as_array().unwrap();
  assert!(jobs.len() >= 3);
  // Newest first, and the admin rows carry the owning account. (The list spans ALL
  // accounts — concurrent test files add rows too — so assert presence, not position.)
  assert!(jobs.iter().any(|j| j["account_id"] == account_id.as_str()));
  // Ordering is sid DESC while created_at is tx-START time — concurrent test commits can
  // invert adjacent rows by milliseconds, so allow that jitter.
  let created = |j: &serde_json::Value| chrono::DateTime::parse_from_rfc3339(j["created_at"].as_str().unwrap()).unwrap();
  assert!(created(&jobs[0]) >= created(&jobs[1]) - chrono::Duration::seconds(2));
}

#[tokio::test]
async fn upload_requires_membership_and_valid_image() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;

  // Empty body is a 400 before storage is even consulted.
  let resp = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri(format!("/v1/files?account_id={account_id}"))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

  // Garbage bytes: unrecognized format, 400.
  let resp = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri(format!("/v1/files?account_id={account_id}"))
        .header(header::COOKIE, &cookie)
        .body(Body::from(vec![0u8; 64]))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

  // Foreign account scope: 404.
  let (_other_cookie, _ou, other_account) = signup_owner(&app, &pool).await;
  let resp = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri(format!("/v1/files?account_id={other_account}"))
        .header(header::COOKIE, &cookie)
        .body(Body::from(vec![0u8; 64]))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// dry_run on EVERY generation endpoint — the 4 model-specific planes + the Any plane.
/// Nothing reaches OpenAI: the job is born terminal (`canceled` / `dry_run`) and the
/// exact would-be payload(s) land in the outbound request log (`provider_calls`).
#[tokio::test]
async fn dry_run_on_every_generation_endpoint() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;

  let model_slugs = [
    ("openai_gpt_image_1", "gpt-image-1"),
    ("openai_gpt_image_1_mini", "gpt-image-1-mini"),
    ("openai_gpt_image_1_5", "gpt-image-1.5"),
    ("openai_gpt_image_2", "gpt-image-2"),
  ];

  for (slug, provider_model_id) in model_slugs {
    let resp = app
      .clone()
      .oneshot(req(
        "POST",
        &format!("/v1/models/{slug}/generate"),
        Some(json!({ "account_id": account_id, "prompt": "dry run probe", "dry_run": true })),
        Some(&cookie),
      ))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "{slug}");
    let job = body_json(resp).await;
    assert_eq!(job["status"], "canceled", "{slug}");
    assert_eq!(job["dry_run"], true, "{slug}");
    assert_eq!(job["failure_code"], "dry_run", "{slug}");
    assert!(job["completed_at"].is_string(), "{slug}: dry-run jobs are terminal immediately");

    // The outbound request log holds the EXACT provider payload, flagged dry-run.
    let job_id: ids::JobId = job["id"].as_str().unwrap().parse().unwrap();
    let calls = db_queries::provider_calls::list_provider_calls_for_job(&pool, &job_id).await.unwrap();
    assert_eq!(calls.len(), 1, "{slug}");
    assert!(calls[0].is_dry_run, "{slug}");
    assert!(calls[0].provider_job_id.is_none(), "{slug}: never sent, no provider handle");
    let body = calls[0].request_body.as_ref().unwrap();
    assert_eq!(body["tools"][0]["model"], provider_model_id, "{slug}");
    assert_eq!(body["background"], true, "{slug}");
  }

  // The Any plane, fanned out: n=3 → 3 canceled slices + 3 dry-run payloads logged.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image",
      Some(json!({
        "model": "openai_gpt_image_1_mini",
        "account_id": account_id,
        "prompt": "dry run fan-out probe",
        "n": 3,
        "dry_run": true
      })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let job = body_json(resp).await;
  assert_eq!(job["status"], "canceled");
  assert_eq!(job["dry_run"], true);
  assert_eq!(job["progress"]["total"], 3);
  assert_eq!(job["progress"]["completed"], 3, "dry-run slices are terminal immediately");
  assert_eq!(job["progress"]["succeeded"], 0);

  let job_id: ids::JobId = job["id"].as_str().unwrap().parse().unwrap();
  let slices = db_queries::job_slices::list_slices_for_job(&pool, &job_id).await.unwrap();
  assert_eq!(slices.len(), 3);
  assert!(slices.iter().all(|s| s.status == enums::JobSliceStatus::Canceled));
  let calls = db_queries::provider_calls::list_provider_calls_for_job(&pool, &job_id).await.unwrap();
  assert_eq!(calls.len(), 3);
  assert!(calls.iter().all(|c| c.is_dry_run && c.job_slice_id.is_some()));

  // And a dry-run job never gets picked up by the dispatcher's claim query.
  let claimed = db_queries::jobs::claim_due_dispatch_jobs(&pool, "openai", 1.0, 100).await.unwrap();
  assert!(claimed.iter().all(|j| !j.is_dry_run), "terminal dry-run jobs must not be claimable");
}

/// The cost twins accept dry_run too (it's simply irrelevant there — nothing is committed
/// either way), so form-built request bodies work on both endpoints unchanged.
#[tokio::test]
async fn cost_twin_accepts_dry_run_flag() {
  let (app, pool) = app_and_pool().await;
  let (cookie, _user_id, account_id) = signup_owner(&app, &pool).await;
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/any/image/cost",
      Some(json!({ "model": "openai_gpt_image_1", "account_id": account_id, "prompt": "x", "dry_run": true })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  assert!(body_json(resp).await["amount_micros"].as_i64().unwrap() > 0);
}
