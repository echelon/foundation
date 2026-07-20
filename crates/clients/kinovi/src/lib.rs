//! Kinovi (seedance2pro) client — ByteDance **Seedance 2.0 / Fast / Mini** video.
//!
//! Async: `POST /api/v1/jobs/createTask` returns a `taskId`; poll
//! `GET /api/v1/jobs/recordInfo?taskId=…` until `status` is `success`/`fail`. Auth via
//! `Authorization: Bearer`. Output video URL is at `output[0].url`. Webhooks via `callBackUrl`.

use client_core::{http_client, read_json, Asset, JobState, Result};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://kinovi.ai/api/v1";

pub struct KinoviClient {
    http: reqwest::Client,
    api_key: String,
}

/// A Seedance video generation request. Text-to-video omits `image_urls`;
/// image-to-video passes the first frame as a public URL.
pub struct VideoRequest<'a> {
    /// Kinovi model id: `seedance-20`, `seedance2-fast`, `seedance2.0-mini`.
    pub model_id: &'a str,
    pub prompt: &'a str,
    /// Seconds, "4".."15".
    pub duration: u32,
    /// "480p" | "720p" | "1080p" | "4k".
    pub output_resolution: &'a str,
    /// "16:9" | "9:16" | "1:1" | "4:3" | "3:4" | "21:9".
    pub aspect_ratio: Option<&'a str>,
    /// First-frame / reference image URLs (image-to-video). Max 9.
    pub image_urls: Vec<String>,
    /// Optional completion webhook.
    pub callback_url: Option<String>,
}

/// A polled task's resolved state + video (when succeeded).
pub struct TaskStatus {
    pub state: JobState,
    pub video: Option<Asset>,
    pub credits_used: Option<f64>,
    pub error: Option<String>,
}

impl KinoviClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: http_client("foundation/kinovi-client"),
            api_key: api_key.into(),
        }
    }

    /// Submit a generation; returns the provider `taskId` to poll.
    pub async fn create_task(&self, req: VideoRequest<'_>) -> Result<String> {
        let inputs = Inputs {
            prompt: req.prompt.to_string(),
            duration: req.duration.to_string(),
            output_resolution: req.output_resolution.to_string(),
            aspect_ratio: req.aspect_ratio.map(str::to_string),
            mode: if req.image_urls.is_empty() {
                None
            } else {
                Some("keyframe".into())
            },
            image_urls: if req.image_urls.is_empty() {
                None
            } else {
                Some(req.image_urls)
            },
        };
        let body = CreateTaskRequest {
            model: req.model_id.to_string(),
            inputs,
            call_back_url: req.callback_url,
        };

        let resp = self
            .http
            .post(format!("{BASE_URL}/jobs/createTask"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let parsed: CreateTaskResponse = read_json(resp).await?;
        Ok(parsed.task_id)
    }

    /// Poll a task. Maps Kinovi's `waiting/generating/success/fail` to [`JobState`].
    pub async fn poll(&self, task_id: &str) -> Result<TaskStatus> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/jobs/recordInfo"))
            .query(&[("taskId", task_id)])
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let info: RecordInfo = read_json(resp).await?;

        let state = match info.status.as_str() {
            "waiting" => JobState::Pending,
            "generating" => JobState::Running,
            "success" => JobState::Succeeded,
            _ => JobState::Failed,
        };
        let video = info
            .output
            .into_iter()
            .flatten()
            .find_map(|o| o.url)
            .map(|url| Asset::Url {
                mime: Some("video/mp4".into()),
                url,
            });

        Ok(TaskStatus {
            state,
            video,
            credits_used: info.credits_used,
            error: info.error.map(|e| e.message),
        })
    }
}

/* ── wire types ── */

#[derive(Serialize)]
struct CreateTaskRequest {
    model: String,
    inputs: Inputs,
    #[serde(rename = "callBackUrl", skip_serializing_if = "Option::is_none")]
    call_back_url: Option<String>,
}

#[derive(Serialize)]
struct Inputs {
    prompt: String,
    duration: String,
    #[serde(rename = "outputResolution")]
    output_resolution: String,
    #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(rename = "imageUrls", skip_serializing_if = "Option::is_none")]
    image_urls: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CreateTaskResponse {
    #[serde(rename = "taskId")]
    task_id: String,
}

#[derive(Deserialize)]
struct RecordInfo {
    status: String,
    output: Option<Vec<OutputItem>>,
    #[serde(rename = "creditsUsed")]
    credits_used: Option<f64>,
    error: Option<ProviderError>,
}

#[derive(Deserialize)]
struct OutputItem {
    url: Option<String>,
}

#[derive(Deserialize)]
struct ProviderError {
    #[allow(dead_code)]
    code: Option<serde_json::Value>,
    #[serde(default)]
    message: String,
}
