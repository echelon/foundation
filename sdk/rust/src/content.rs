use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CACHE_CONTROL, CONTENT_TYPE};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    ArtifactRef, AudioOutput, ChatResult, Error, ImageOutputs, MeshOutput, TokenCraftClient,
    VideoOutput,
};

const MAX_ARTIFACT_BYTES: u32 = 8 * 1024 * 1024;
const SHA256_HEADER: &str = "x-tokencraft-content-sha256";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentPhase {
    Processing,
    Available,
    Archiving,
    Archived,
    Removed,
    Expired,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ContentMetadata {
    pub phase: ContentPhase,
    #[serde(default)]
    pub media: bool,
    pub archive_at: Option<String>,
    pub expires_at: Option<String>,
    pub restore: Option<RestoreProgress>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreState {
    Pending,
    Ready,
    Failed,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestoreProgress {
    pub state: RestoreState,
    pub ready_objects: u8,
    pub total_objects: u8,
    pub requested_at: String,
    pub retry_at: String,
}

#[derive(Clone, Deserialize)]
pub struct RetainedContent {
    #[serde(flatten)]
    pub metadata: ContentMetadata,
    /// The original chat or connector input as recorded by the gateway.
    pub input: Option<serde_json::Value>,
    pub result: Option<RetainedResult>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum RetainedResult {
    Chat(ChatResult),
    Images(ImageOutputs),
    Video(VideoOutput),
    Audio(AudioOutput),
    Mesh(MeshOutput),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactLocation {
    Warm,
    Archived,
}

pub struct VerifiedArtifact {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

impl TokenCraftClient {
    /// Read saved dates and any restore progress without fetching request content.
    pub async fn content_metadata(&self, id: &str) -> Result<ContentMetadata, Error> {
        let request = self.request_for(Method::GET, id, "/content")?
            .query(&[("metadata_only", true)])
            .header(CACHE_CONTROL, "no-store");
        self.send(request, Some(id)).await
    }

    /// Read the input and any result available during the warm retention window.
    pub async fn warm_content(&self, id: &str) -> Result<RetainedContent, Error> {
        let request = self.request_for(Method::GET, id, "/content")?
            .header(CACHE_CONTROL, "no-store");
        self.send(request, Some(id)).await
    }

    /// Read an available archive. A cold object may return `archive_retrieval_required`.
    pub async fn retrieve_content(&self, id: &str) -> Result<RetainedContent, Error> {
        let request = self.request_for(Method::POST, id, "/content/retrieve")?
            .header(CACHE_CONTROL, "no-store");
        self.send(request, Some(id)).await
    }

    /// Download a stored output from the gateway and verify it against its saved reference.
    pub async fn download_artifact(
        &self,
        id: &str,
        artifact: &ArtifactRef,
        location: ArtifactLocation,
    ) -> Result<VerifiedArtifact, Error> {
        if !(0..8).contains(&artifact.slot)
            || !(1..=MAX_ARTIFACT_BYTES).contains(&artifact.byte_length)
            || artifact.sha256.len() != 64
            || !artifact.sha256.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(Error::InvalidInput("supply a valid saved artifact reference"));
        }
        let prefix = match location {
            ArtifactLocation::Warm => "/artifacts",
            ArtifactLocation::Archived => "/content/artifacts",
        };
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/octet-stream"));
        // Only gateway paths built from validated identifiers receive credentials.
        let response = self
            .request_for(Method::GET, id, &format!("{prefix}/{}", artifact.slot))?
            .headers(headers)
            .header(CACHE_CONTROL, "no-store")
            .send()
            .await
            .map_err(|source| Error::from_http(source, Some(id.to_owned())))?;
        let (mut response, request_id) = Self::checked_response(response, Some(id)).await?;
        let response_status = response.status();
        let invalid = || Error::Gateway {
            status: response_status,
            code: "invalid_artifact".to_owned(),
            message: "the gateway artifact did not match its saved reference".to_owned(),
            request_id: request_id.clone(),
        };
        let limit = artifact.byte_length as usize;
        let header = |name| response.headers().get(name).and_then(|value| value.to_str().ok());
        let mime = header(CONTENT_TYPE.as_str())
            .filter(|value| matches!(*value, "image/png" | "video/mp4" | "audio/wav" | "model/gltf-binary"))
            .ok_or_else(invalid)?
            .to_owned();
        if response_status != StatusCode::OK
            || header(SHA256_HEADER) != Some(artifact.sha256.as_str())
            || response.content_length().is_some_and(|length| length != limit as u64)
        {
            return Err(invalid());
        }
        let mut bytes = Vec::with_capacity(limit);
        let mut checksum = Sha256::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|source| Error::from_http(source, request_id.clone()))?
        {
            if chunk.len() > limit - bytes.len() {
                return Err(invalid());
            }
            checksum.update(&chunk);
            bytes.extend_from_slice(&chunk);
        }
        if bytes.len() != limit || format!("{:x}", checksum.finalize()) != artifact.sha256 {
            return Err(invalid());
        }
        Ok(VerifiedArtifact { content_type: mime, bytes })
    }
}
