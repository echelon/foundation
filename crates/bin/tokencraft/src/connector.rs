use std::{fs, io::ErrorKind, path::PathBuf, sync::Arc};

use anyhow::{ensure, Context, Result};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use provider_mapping::{Direction, Mapping, Registry, Status};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

use crate::state::write_private;

const PROVIDER: &str = "self_hosted";
const MODEL: &str = "example-image";
const EXAMPLE_MAPPING: &[u8] =
    include_bytes!("../../../../examples/mapping/example-image.map.json");

pub(crate) fn router(directory: PathBuf, token: &str) -> Result<Router> {
    let registry = Registry::builtin()?;
    let mapping = mapping(&registry)?;
    let fixture = Arc::new(Fixture {
        directory,
        registry,
        mapping,
        authorization: Sha256::digest(format!("Bearer {token}").as_bytes()).into(),
        submissions: Mutex::new(()),
    });
    Ok(Router::new()
        .route("/v1/jobs", post(submit))
        .route("/v1/jobs/:handle", get(poll))
        .route("/v1/jobs/:handle/artifacts/:slot", get(artifact))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(fixture))
}

fn mapping(registry: &Registry) -> Result<Mapping> {
    let mapping = Mapping::parse(EXAMPLE_MAPPING)?;
    mapping.validate(registry)?;
    ensure!(
        mapping.status == Status::Approved
            && mapping.provider == PROVIDER
            && mapping.model == MODEL,
        "example mapping needs approval and must match the bundled offer"
    );
    Ok(mapping)
}

struct Fixture {
    directory: PathBuf,
    registry: Registry,
    mapping: Mapping,
    authorization: [u8; 32],
    submissions: Mutex<()>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    version: u8,
    request_id: String,
    model: String,
    input: ImageInput,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ImageInput {
    Image {
        prompt: String,
        count: u8,
        aspect_ratio: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureInput {
    description: String,
    outputs: u8,
    layout: Shape,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Job {
    digest: String,
    count: u8,
    shape: Shape,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Shape {
    Square,
    Landscape,
}

impl Shape {
    fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Square => (64, 64),
            Self::Landscape => (64, 48),
        }
    }
}

#[derive(Serialize)]
struct SubmissionReply {
    version: u8,
    request_id: String,
    outcome: Outcome,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Accepted { handle: String },
    Rejected,
    Uncertain,
}

#[derive(Serialize)]
struct PollReply {
    version: u8,
    handle: String,
    status: &'static str,
}

async fn submit(
    State(fixture): State<Arc<Fixture>>,
    headers: HeaderMap,
    Json(submission): Json<Submission>,
) -> Response {
    if !fixture.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok());
    if submission.version != 1
        || !valid_request_id(&submission.request_id)
        || idempotency_key != Some(submission.request_id.as_str())
    {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let _lock = fixture.submissions.lock().await;
    let outcome = fixture.record(&submission).unwrap_or(Outcome::Uncertain);
    Json(SubmissionReply {
        version: 1,
        request_id: submission.request_id,
        outcome,
    })
    .into_response()
}

async fn poll(
    State(fixture): State<Arc<Fixture>>,
    Path(handle): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !fixture.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match fixture.find(&handle) {
        Ok(Some(_)) => Json(PollReply {
            version: 1,
            handle,
            status: "succeeded",
        })
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn artifact(
    State(fixture): State<Arc<Fixture>>,
    Path((handle, slot)): Path<(String, u8)>,
    headers: HeaderMap,
) -> Response {
    if !fixture.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let job = match fixture.find(&handle) {
        Ok(Some(job)) if slot < job.count => job,
        Ok(_) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    match png(job.shape, &handle, slot) {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/png")], bytes).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

impl Fixture {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        let value = headers
            .get(header::AUTHORIZATION)
            .map_or(&[][..], |header| header.as_bytes());
        let digest: [u8; 32] = Sha256::digest(value).into();
        bool::from(self.authorization.ct_eq(&digest))
    }

    fn record(&self, submission: &Submission) -> Result<Outcome> {
        let Ok(payload) = self.mapping.apply(
            &self.registry,
            PROVIDER,
            &submission.model,
            Direction::Request,
            &serde_json::to_value(&submission.input)?,
        ) else {
            return Ok(Outcome::Rejected);
        };
        let input: FixtureInput =
            serde_json::from_value(payload).context("invalid example mapping output")?;
        if input.description.trim().is_empty() || input.description.len() > 4096 {
            return Ok(Outcome::Rejected);
        }
        let handle = hex::encode(Sha256::digest(submission.request_id.as_bytes()));
        let job = Job {
            digest: hex::encode(Sha256::digest(serde_json::to_vec(submission)?)),
            count: input.outputs,
            shape: input.layout,
        };
        match self.find(&handle)? {
            Some(existing) if existing.digest != job.digest => return Ok(Outcome::Uncertain),
            Some(_) => {}
            None => write_private(&self.directory.join(&handle), &serde_json::to_vec(&job)?)?,
        }
        Ok(Outcome::Accepted { handle })
    }

    fn find(&self, handle: &str) -> Result<Option<Job>> {
        if handle.len() != 64 || !handle.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(None);
        }
        match fs::read(self.directory.join(handle)) {
            Ok(contents) => Ok(Some(
                serde_json::from_slice(&contents).context("invalid fixture job")?,
            )),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("read fixture job"),
        }
    }
}

fn valid_request_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn png(shape: Shape, handle: &str, slot: u8) -> image::ImageResult<Vec<u8>> {
    let color = Sha256::digest(format!("{handle}/{slot}").as_bytes());
    let (width, height) = shape.dimensions();
    let pixels = color[..3].repeat((width * height) as usize);
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes).write_image(&pixels, width, height, ExtendedColorType::Rgb8)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{private_dir, random_secret};

    #[test]
    fn fixture_keeps_handle_and_png_across_restart() {
        struct Scratch(PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let scratch =
            Scratch(std::env::temp_dir().join(format!("tokencraft-{}", random_secret().unwrap())));
        private_dir(&scratch.0).unwrap();
        let reopen = || {
            let registry = Registry::builtin().unwrap();
            let mapping = mapping(&registry).unwrap();
            Fixture {
                directory: scratch.0.clone(),
                registry,
                mapping,
                authorization: [0; 32],
                submissions: Mutex::new(()),
            }
        };
        let mut request: Submission = serde_json::from_str(
            r#"{
            "version": 1, "request_id": "req_fixture", "model": "example-image",
            "input": {"kind": "image", "prompt": "sample", "count": 2, "aspect_ratio": "4:3"}
        }"#,
        )
        .unwrap();
        let first = reopen().record(&request).unwrap();
        assert_eq!(reopen().record(&request).unwrap(), first);
        let Outcome::Accepted { handle } = first else {
            panic!("expected a fixture handle");
        };
        let fixture = reopen();
        let job = fixture.find(&handle).unwrap().unwrap();
        assert_eq!(job.count, 2);
        assert!(fixture.find("../../dev-keys.json").unwrap().is_none());
        let bytes = png(job.shape, &handle, 0).unwrap();
        let result = image::load_from_memory(&bytes).unwrap();
        assert_eq!((result.width(), result.height()), (64, 48));
        let ImageInput::Image { count, .. } = &mut request.input;
        *count = 3;
        assert_eq!(fixture.record(&request).unwrap(), Outcome::Uncertain);
    }
}
