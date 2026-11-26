//! In-process HTTP fixtures. The test binaries import the helpers they need.
#![allow(dead_code)]

use std::sync::Arc;

use api_gateway::{auth::token::JwtKeys, AppState};
use axum::{
  body::{self, Body},
  http::{header, Method, Request},
  response::Response,
  Router,
};
use serde_json::Value;

pub const JWT_SECRET: &str = "test-secret";

pub fn test_state(pool: db::PgPool) -> AppState {
  AppState {
    pool,
    jwt: Arc::new(JwtKeys::new(JWT_SECRET)),
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
  }
}

pub async fn app_and_pool() -> (Router, db::PgPool) {
  let pool = db::test_fixtures::create_test_pool().await;
  (api_gateway::build_router(test_state(pool.clone())), pool)
}

pub fn unique_email() -> String {
  format!("user-{}@example.com", id_core::mint("t"))
}

pub fn session_cookie(response: &Response) -> Option<String> {
  response.headers().get_all(header::SET_COOKIE).iter().find_map(|value| {
    let cookie = value.to_str().ok()?.split(';').next()?;
    let token = cookie.strip_prefix("foundation_session=")?;
    (!token.is_empty()).then(|| cookie.to_owned())
  })
}

pub async fn body_json(response: Response) -> Value {
  let status = response.status();
  let bytes = body::to_bytes(response.into_body(), usize::MAX)
    .await
    .expect("read test response body");
  serde_json::from_slice(&bytes)
    .unwrap_or_else(|error| panic!("expected JSON in HTTP {status} response: {error}"))
}

pub fn get(uri: &str, cookie: Option<&str>) -> Request<Body> {
  request(Method::GET, uri, cookie).body(Body::empty()).unwrap()
}

pub fn delete(uri: &str, cookie: Option<&str>) -> Request<Body> {
  request(Method::DELETE, uri, cookie).body(Body::empty()).unwrap()
}

pub fn post_empty(uri: &str, cookie: Option<&str>) -> Request<Body> {
  request(Method::POST, uri, cookie).body(Body::empty()).unwrap()
}

pub fn post(uri: &str, payload: Value, cookie: Option<&str>) -> Request<Body> {
  request(Method::POST, uri, cookie)
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(payload.to_string()))
    .unwrap()
}

pub fn put(uri: &str, payload: Value, cookie: Option<&str>) -> Request<Body> {
  request(Method::PUT, uri, cookie)
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(payload.to_string()))
    .unwrap()
}

fn request(method: Method, uri: &str, cookie: Option<&str>) -> axum::http::request::Builder {
  let builder = Request::builder().method(method).uri(uri);
  match cookie {
    Some(cookie) => builder.header(header::COOKIE, cookie),
    None => builder,
  }
}
