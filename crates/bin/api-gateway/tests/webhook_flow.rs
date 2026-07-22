//! End-to-end `/v1/webhooks/openai`: signature verification (Standard Webhooks), the
//! every-delivery inbound log, correlation back to the originating request/job, and
//! at-least-once dedup. Provider processing itself isn't exercised here (no OpenAI in
//! tests) — a matched delivery whose processing can't run logs `error` and stays safe
//! behind the poll loop.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::Row;
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;
use openai_client::api::webhooks::{sign_webhook_payload, SignWebhookArgs};
use openai_client::creds::openai_webhook_secret::OpenAiWebhookSecret;

/// base64("test-secret-key") — a known secret both the app state and the test signer use.
const TEST_SECRET: &str = "whsec_dGVzdC1zZWNyZXQta2V5";

async fn app_and_pool(with_secret: bool) -> (Router, db::PgPool) {
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
    openai_webhook_secret: with_secret.then(|| Arc::new(OpenAiWebhookSecret::new(TEST_SECRET).unwrap())),
  };
  (api_gateway::build_router(state), pool)
}

fn secret() -> OpenAiWebhookSecret {
  OpenAiWebhookSecret::new(TEST_SECRET).unwrap()
}

/// A signed webhook request, exactly as OpenAI's sender would produce it.
fn webhook_req(webhook_id: &str, body: &str, signature: &str) -> Request<Body> {
  Request::builder()
    .method("POST")
    .uri("/v1/webhooks/openai")
    .header(header::CONTENT_TYPE, "application/json")
    .header("webhook-id", webhook_id)
    .header("webhook-timestamp", "1753142400")
    .header("webhook-signature", signature)
    .body(Body::from(body.to_string()))
    .unwrap()
}

fn sign(webhook_id: &str, body: &str) -> String {
  let s = secret();
  sign_webhook_payload(&SignWebhookArgs {
    secret: &s,
    webhook_id,
    webhook_timestamp: "1753142400",
    body,
  })
}

fn event_body(event_id: &str, kind: &str, resp_id: &str) -> String {
  json!({ "object": "event", "id": event_id, "type": kind, "created_at": 1753142400, "data": { "id": resp_id } })
    .to_string()
}

async fn body_json(resp: axum::response::Response) -> Value {
  let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
  serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// The latest inbound_webhooks row for an event id.
async fn latest_log(pool: &db::PgPool, event_id: Option<&str>) -> sqlx::postgres::PgRow {
  // Test-side peek — raw query keeps the sqlx offline cache out of test-only SQL.
  sqlx::query(
    "SELECT provider, event_id, event_type, object_id, is_signature_valid, outcome,
            provider_call_id, job_id, job_slice_id
     FROM inbound_webhooks
     WHERE ($1::text IS NULL OR event_id = $1)
     ORDER BY sid DESC LIMIT 1",
  )
  .bind(event_id)
  .fetch_one(pool)
  .await
  .unwrap()
}

#[tokio::test]
async fn invalid_signature_is_rejected_and_logged() {
  let (app, pool) = app_and_pool(true).await;
  let evt = format!("evt_{}", id_core::mint("t"));
  let body = event_body(&evt, "response.completed", "resp_bogus_sig");

  let resp = app
    .clone()
    .oneshot(webhook_req("wh_1", &body, "v1,AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

  // Logged anyway — rejected deliveries are security signal. The identifying fields are
  // captured from the (untrusted) payload so the row is findable.
  let row = latest_log(&pool, Some(&evt)).await;
  assert!(!row.get::<bool, _>("is_signature_valid"));
  assert_eq!(row.get::<String, _>("outcome"), "signature_failed");
  assert_eq!(row.get::<Option<String>, _>("object_id").as_deref(), Some("resp_bogus_sig"));
}

#[tokio::test]
async fn missing_secret_config_refuses_but_logs() {
  let (app, pool) = app_and_pool(false).await;
  let evt = format!("evt_{}", id_core::mint("t"));
  let body = event_body(&evt, "response.completed", "resp_no_secret");

  let resp = app.clone().oneshot(webhook_req("wh_1", &body, &sign("wh_1", &body))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
  let row = latest_log(&pool, Some(&evt)).await;
  assert!(!row.get::<bool, _>("is_signature_valid"));
  assert_eq!(row.get::<String, _>("outcome"), "not_configured");
}

#[tokio::test]
async fn verified_but_unknown_subject_is_logged_unmatched() {
  let (app, pool) = app_and_pool(true).await;
  let evt = format!("evt_{}", id_core::mint("t"));
  let resp_id = format!("resp_unknown_{}", id_core::mint("t"));
  let body = event_body(&evt, "response.completed", &resp_id);

  let resp = app.clone().oneshot(webhook_req("wh_1", &body, &sign("wh_1", &body))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  assert_eq!(body_json(resp).await["outcome"], "unmatched");

  let row = latest_log(&pool, Some(&evt)).await;
  assert!(row.get::<bool, _>("is_signature_valid"));
  assert_eq!(row.get::<String, _>("outcome"), "unmatched");
  assert_eq!(row.get::<Option<String>, _>("object_id").as_deref(), Some(resp_id.as_str()));
  assert!(row.get::<Option<String>, _>("job_id").is_none());
}

#[tokio::test]
async fn redelivery_of_a_handled_event_is_deduplicated() {
  let (app, pool) = app_and_pool(true).await;
  let evt = format!("evt_{}", id_core::mint("t"));
  let body = event_body(&evt, "response.completed", "resp_dup_probe");
  let signature = sign("wh_1", &body);

  let first = app.clone().oneshot(webhook_req("wh_1", &body, &signature)).await.unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  assert_eq!(body_json(first).await["outcome"], "unmatched");

  let second = app.clone().oneshot(webhook_req("wh_1", &body, &signature)).await.unwrap();
  assert_eq!(second.status(), StatusCode::OK);
  assert_eq!(body_json(second).await["outcome"], "duplicate");

  // Both deliveries logged — "log every inbound webhook" includes redeliveries.
  let n: i64 = sqlx::query_scalar("SELECT count(*) FROM inbound_webhooks WHERE event_id = $1")
    .bind(&evt)
    .fetch_one(&pool)
    .await
    .unwrap();
  assert_eq!(n, 2);
}

#[tokio::test]
async fn non_response_events_are_verified_then_ignored() {
  let (app, pool) = app_and_pool(true).await;
  let evt = format!("evt_{}", id_core::mint("t"));
  let body = json!({ "object": "event", "id": evt, "type": "batch.completed", "created_at": 1, "data": { "id": "batch_9" } })
    .to_string();

  let resp = app.clone().oneshot(webhook_req("wh_1", &body, &sign("wh_1", &body))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  assert_eq!(body_json(resp).await["outcome"], "ignored");
  let row = latest_log(&pool, Some(&evt)).await;
  assert_eq!(row.get::<String, _>("outcome"), "ignored");
  assert_eq!(row.get::<Option<String>, _>("event_type").as_deref(), Some("batch.completed"));
}

/// Correlation: a webhook about a KNOWN provider handle finds the job and the originating
/// outbound request (the request_id). With no OpenAI credential in tests the processing
/// step can't run — outcome `error`, correlation ids still logged, poll loop's problem.
#[tokio::test]
async fn matched_delivery_correlates_to_job_and_request_log() {
  let (app, pool) = app_and_pool(true).await;

  // A real enqueued job (via the API) that "reached" the provider: simulate the
  // dispatcher's bookkeeping directly — accepted with a known resp_… handle + pcall row.
  let cookie_jar = signup(&app).await;
  let account_id = first_account(&pool, &cookie_jar.1).await;
  let enq = app
    .clone()
    .oneshot(api_req(
      "POST",
      "/v1/any/image",
      Some(json!({ "model": "openai_gpt_image_v1", "account_id": account_id, "prompt": "correlate me" })),
      Some(&cookie_jar.0),
    ))
    .await
    .unwrap();
  assert_eq!(enq.status(), StatusCode::CREATED);
  let job_id: ids::JobId = body_json(enq).await["id"].as_str().unwrap().parse().unwrap();

  let resp_handle = format!("resp_corr_{}", id_core::mint("t"));
  let claimed = db_queries::jobs::claim_due_dispatch_jobs(&pool, 300.0, 100).await.unwrap();
  assert!(claimed.iter().any(|j| j.id == job_id), "job should be dispatch-claimable");
  let call = db_queries::provider_calls::create_provider_call(
    &pool,
    db_queries::provider_calls::NewProviderCall {
      job_id: &job_id,
      job_slice_id: None,
      provider: "openai",
      url: "https://api.openai.com/v1/responses",
      request_body: &json!({ "model": "gpt-5.4-nano" }),
      is_dry_run: false,
    },
  )
  .await
  .unwrap();
  db_queries::provider_calls::set_provider_call_result(
    &pool,
    db_queries::provider_calls::ProviderCallResult {
      id: &call.id,
      provider_job_id: Some(&resp_handle),
      response_body: None,
      latency_ms: Some(42),
    },
  )
  .await
  .unwrap();
  db_queries::jobs::set_job_provider_accepted(&pool, &job_id, Some(&resp_handle)).await.unwrap();

  let evt = format!("evt_{}", id_core::mint("t"));
  let body = event_body(&evt, "response.completed", &resp_handle);
  let resp = app.clone().oneshot(webhook_req("wh_1", &body, &sign("wh_1", &body))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  assert_eq!(body_json(resp).await["outcome"], "error", "no provider credential in tests");

  let row = latest_log(&pool, Some(&evt)).await;
  assert_eq!(row.get::<Option<String>, _>("job_id").as_deref(), Some(job_id.as_str()));
  assert_eq!(row.get::<Option<String>, _>("provider_call_id").as_deref(), Some(call.id.as_str()));
  assert_eq!(row.get::<String, _>("outcome"), "error");
}

/* ── minimal auth helpers (mirrors generation_flow.rs) ── */

fn api_req(method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
  let mut b = Request::builder().method(method).uri(uri);
  if body.is_some() {
    b = b.header(header::CONTENT_TYPE, "application/json");
  }
  if let Some(c) = cookie {
    b = b.header(header::COOKIE, c);
  }
  match body {
    Some(v) => b.body(Body::from(serde_json::to_vec(&v).unwrap())).unwrap(),
    None => b.body(Body::empty()).unwrap(),
  }
}

async fn signup(app: &Router) -> (String, ids::UserId) {
  let email = format!("user-{}@example.com", id_core::mint("t"));
  let resp = app
    .clone()
    .oneshot(api_req("POST", "/v1/auth/signup", Some(json!({ "email": email, "password": "password1" })), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = resp
    .headers()
    .get_all(header::SET_COOKIE)
    .iter()
    .filter_map(|v| v.to_str().ok())
    .find_map(|s| s.strip_prefix("foundation_session=").map(|r| format!("foundation_session={}", r.split(';').next().unwrap())))
    .unwrap();
  let user_id: ids::UserId = body_json(resp).await["user"]["id"].as_str().unwrap().parse().unwrap();
  (cookie, user_id)
}

async fn first_account(pool: &db::PgPool, user_id: &ids::UserId) -> ids::AccountId {
  db_queries::members::list_memberships_for_user(pool, user_id).await.unwrap()[0].account_id.clone()
}
