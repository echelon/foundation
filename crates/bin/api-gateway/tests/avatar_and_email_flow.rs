//! End-to-end avatar + email-verification flows through the REAL router against
//! `foundation_test` — every request runs the full axum stack (routing, CORS, body limits,
//! cookie middleware, the online session check) via `tower::ServiceExt::oneshot`.
//!
//! Fixtures sign up real users over HTTP and act with their session cookie. Where a flow
//! depends on the outside world (gravatar.com, R2), the test plants the db rows the
//! background job would have written and exercises everything else for real — the genuinely
//! unreachable parts are documented in `untested.md`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;
use db_queries::{user_avatar_files, user_avatars};
use enums::AvatarFileSource;

async fn app_and_pool() -> (Router, db::PgPool) {
  let pool = db::test_fixtures::create_test_pool().await;
  let state = AppState {
    pool: pool.clone(),
    jwt: Arc::new(JwtKeys::new("test-secret")),
    cookie_domain: "localhost".into(),
    cookie_secure: false,
    app_base_url: "http://localhost:3000".into(),
    email_from: "Foundation <noreply@localhost>".into(),
    resend_api_key: None, // email sends record + skip (no provider call, no mock)
    bucket: None,         // R2 off — upload endpoint untestable here (see untested.md)
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: None,
    google: None,
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
  let body = match body {
    Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
    None => Body::empty(),
  };
  b.body(body).unwrap()
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

/// Sign up a fresh user through the real endpoint; returns (cookie, user_id).
async fn signup(app: &Router) -> (String, ids::UserId) {
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/auth/signup",
      Some(json!({ "email": unique_email(), "password": "hunter2hunter2" })),
      None,
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).expect("signup sets the session cookie");
  let body = body_json(resp).await;
  let user_id: ids::UserId = body["user"]["id"].as_str().unwrap().parse().unwrap();
  (cookie, user_id)
}

async fn me_avatar(app: &Router, cookie: &str) -> Value {
  let resp = app.clone().oneshot(req("GET", "/v1/auth/me", None, Some(cookie))).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  body_json(resp).await["user"]["avatar"].clone()
}

/// Plant the rows the gravatar-job would have written after mirroring an image.
async fn plant_gravatar_mirror(pool: &db::PgPool, user_id: &ids::UserId) -> ids::UserAvatarFileId {
  plant_file(pool, user_id, AvatarFileSource::GravatarMirror, true).await
}

async fn plant_file(
  pool: &db::PgPool,
  user_id: &ids::UserId,
  source: AvatarFileSource,
  link_as_gravatar: bool,
) -> ids::UserAvatarFileId {
  let ulid = id_core::mint("h");
  let bucket_hash = format!("{}{}", &ulid[2..28], &ulid[2..8]);
  let file = user_avatar_files::create_user_avatar_file(
    pool,
    user_avatar_files::NewUserAvatarFile {
      user_id,
      original_user_avatar_file_id: None,
      avatar_source: source,
      bucket_hash: &bucket_hash,
      content_hash: "cafebabe",
      bucket_suffix: Some("_512"),
      bucket_extension: ".png",
      width: Some(512),
      height: Some(512),
      bytes: 999,
    },
  )
  .await
  .unwrap();
  if link_as_gravatar {
    user_avatars::set_gravatar_mirrored(pool, user_id, &file.id).await.unwrap();
  } else {
    user_avatars::set_custom_avatar(pool, user_id, &file.id).await.unwrap();
  }
  file.id
}

/* ── Gravatar: change (refresh → new mirror) ── */

#[tokio::test]
async fn changing_gravatar_full_flow() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id) = signup(&app).await;

  // Fresh signup: auto preference, poll pending, monogram shows.
  let avatar = me_avatar(&app, &cookie).await;
  assert_eq!(avatar["preferred_source"], "not_yet_set");
  assert_eq!(avatar["source"], "letter");
  assert_eq!(avatar["gravatar_checked"], false);

  // The job mirrors one → it displays automatically (auto-pick) with CDN links.
  plant_gravatar_mirror(&pool, &user_id).await;
  let avatar = me_avatar(&app, &cookie).await;
  assert_eq!(avatar["source"], "gravatar");
  assert_eq!(avatar["has_gravatar"], true);
  let first_url = avatar["links"]["cdn_url"].as_str().unwrap().to_string();
  assert!(first_url.starts_with("http"), "links must be absolute: {first_url}");

  // The user asks for a refresh (the CHANGE) → checked goes pending, old mirror still shows.
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/avatar/gravatar/refresh", None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let avatar = body_json(resp).await["avatar"].clone();
  assert_eq!(avatar["gravatar_checked"], false, "refresh re-requests the poll");
  assert_eq!(avatar["source"], "gravatar", "the current mirror keeps displaying meanwhile");

  // The job re-mirrors (new image) → the links change.
  plant_gravatar_mirror(&pool, &user_id).await;
  let avatar = me_avatar(&app, &cookie).await;
  assert_eq!(avatar["gravatar_checked"], true);
  let second_url = avatar["links"]["cdn_url"].as_str().unwrap();
  assert_ne!(second_url, first_url, "a changed gravatar produces a new CDN URL");
}

/* ── Gravatar: dismiss (prefer away; mirror kept) + remove ── */

#[tokio::test]
async fn dismissing_gravatar_full_flow() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id) = signup(&app).await;
  plant_gravatar_mirror(&pool, &user_id).await;

  // Dismiss = choose the monogram; the mirror stays available.
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "letter" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let avatar = body_json(resp).await["avatar"].clone();
  assert_eq!(avatar["preferred_source"], "letter");
  assert_eq!(avatar["source"], "letter");
  assert_eq!(avatar["has_gravatar"], true, "dismissal keeps the mirror");
  assert!(avatar["links"].is_null(), "monogram has no image links");

  // Re-enable by choosing it again.
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "gravatar" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(body_json(resp).await["avatar"]["source"], "gravatar");

  // Remove the mirror entirely → monogram, and it stays gone (no auto re-poll).
  let resp = app
    .clone()
    .oneshot(req("DELETE", "/v1/auth/avatar/gravatar", None, Some(&cookie)))
    .await
    .unwrap();
  let avatar = body_json(resp).await["avatar"].clone();
  assert_eq!(avatar["has_gravatar"], false);
  assert_eq!(avatar["source"], "letter");
  assert_eq!(avatar["gravatar_checked"], true, "removal must not re-queue a poll");

  // Unknown enum value is rejected by serde at the boundary (422).
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "banana" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/* ── Custom avatar: choose over gravatar, dismiss ──
   (SETTING via the upload endpoint needs R2 — see untested.md; the display/choice/removal
   surface is fully covered by planting the rows the upload would have written.) */

#[tokio::test]
async fn uploaded_avatar_choice_and_dismissal_full_flow() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id) = signup(&app).await;

  // Preferring "custom" with no upload is a 400.
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "custom" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

  // Plant an uploaded avatar (as the upload endpoint records it) + a mirrored gravatar.
  plant_file(&pool, &user_id, AvatarFileSource::CustomUpload, false).await;
  plant_gravatar_mirror(&pool, &user_id).await;

  // Upload set the preference to custom; both options are available.
  let avatar = me_avatar(&app, &cookie).await;
  assert_eq!(avatar["source"], "custom");
  assert_eq!(avatar["has_custom"], true);
  assert_eq!(avatar["has_gravatar"], true);

  // The user can choose the gravatar OVER the upload — and switch back.
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "gravatar" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(body_json(resp).await["avatar"]["source"], "gravatar");
  let resp = app
    .clone()
    .oneshot(req("PUT", "/v1/auth/avatar/preference", Some(json!({ "source": "custom" })), Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(body_json(resp).await["avatar"]["source"], "custom");

  // Dismiss the upload → falls back to the gravatar (auto-pick), pointer cleared.
  let resp = app.clone().oneshot(req("DELETE", "/v1/auth/avatar", None, Some(&cookie))).await.unwrap();
  let avatar = body_json(resp).await["avatar"].clone();
  assert_eq!(avatar["has_custom"], false);
  assert_eq!(avatar["source"], "gravatar", "auto-pick falls back to the mirrored gravatar");
}

/* ── Email change ── */

#[tokio::test]
async fn changing_email_full_flow() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id) = signup(&app).await;

  let new_email = unique_email();
  let resp = app
    .clone()
    .oneshot(req(
      "PUT",
      "/v1/auth/email",
      Some(json!({ "new_email": new_email, "current_password": "hunter2hunter2" })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);

  // /me reflects the new address, unverified.
  let resp = app.clone().oneshot(req("GET", "/v1/auth/me", None, Some(&cookie))).await.unwrap();
  let user = body_json(resp).await["user"].clone();
  assert_eq!(user["email"], new_email);
  assert_eq!(user["email_verified"], false);

  // The email-change log row and the ORG-VISIBLE audit event were written.
  let change_rows: i64 =
    sqlx::query_scalar("SELECT count(*) FROM email_changes WHERE user_id = $1 AND change_source = 'user'")
      .bind(user_id.as_str())
      .fetch_one(&pool)
      .await
      .unwrap();
  assert_eq!(change_rows, 1);
  let (event_count, org_visible): (i64, bool) = sqlx::query_as(
    "SELECT count(*), bool_and(is_org_visible) FROM user_audit_logs WHERE user_id = $1 AND event = 'email_changed'",
  )
  .bind(user_id.as_str())
  .fetch_one(&pool)
  .await
  .unwrap();
  assert_eq!(event_count, 1);
  assert!(org_visible, "email_changed must be visible at the org/account level");

  // Wrong password is a 401 and changes nothing.
  let resp = app
    .clone()
    .oneshot(req(
      "PUT",
      "/v1/auth/email",
      Some(json!({ "new_email": unique_email(), "current_password": "wrong" })),
      Some(&cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/* ── Send verification email + confirm ──
   (The actual Resend delivery is not exercised — key is None, so the send records + skips;
   see untested.md. Everything else is the real path.) */

#[tokio::test]
async fn send_verification_email_full_flow() {
  let (app, pool) = app_and_pool().await;
  let (cookie, user_id) = signup(&app).await;

  // Request the verification email.
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/email/verification", None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);

  // A verification row + an email_sends log row exist.
  let token_rows: i64 =
    sqlx::query_scalar("SELECT count(*) FROM email_verifications WHERE user_id = $1 AND validated_at IS NULL")
      .bind(user_id.as_str())
      .fetch_one(&pool)
      .await
      .unwrap();
  assert_eq!(token_rows, 1);
  let send_rows: i64 =
    sqlx::query_scalar("SELECT count(*) FROM email_sends WHERE user_id = $1 AND template = 'email_verify'")
      .bind(user_id.as_str())
      .fetch_one(&pool)
      .await
      .unwrap();
  assert_eq!(send_rows, 1);

  // We can't read the emailed plaintext token (only its hash is stored) — plant a known one,
  // exactly as the email link would deliver it.
  let token = format!("e2e-{}", id_core::mint("tok"));
  let token_hash = {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(token.as_bytes()))
  };
  sqlx::query("UPDATE email_verifications SET token_hash = $1 WHERE user_id = $2 AND validated_at IS NULL")
    .bind(&token_hash)
    .bind(user_id.as_str())
    .execute(&pool)
    .await
    .unwrap();

  // Confirm — PUBLIC endpoint, no cookie: the token is the proof.
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/email/verification/confirm", Some(json!({ "token": token })), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);

  // Verified now, with the ORG-VISIBLE audit event.
  let resp = app.clone().oneshot(req("GET", "/v1/auth/me", None, Some(&cookie))).await.unwrap();
  assert_eq!(body_json(resp).await["user"]["email_verified"], true);
  let (event_count, org_visible): (i64, bool) = sqlx::query_as(
    "SELECT count(*), bool_and(is_org_visible) FROM user_audit_logs WHERE user_id = $1 AND event = 'email_verified'",
  )
  .bind(user_id.as_str())
  .fetch_one(&pool)
  .await
  .unwrap();
  assert_eq!(event_count, 1);
  assert!(org_visible, "email_verified must be visible at the org/account level");

  // Re-requesting while verified is a 400.
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/email/verification", None, Some(&cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
