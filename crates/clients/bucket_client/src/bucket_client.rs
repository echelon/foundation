use hmac::Mac;
use s3::command::Command;
use s3::error::S3Error;
use s3::request::{tokio_backend::ReqwestRequest, Request};
use s3::signing;
use s3::Bucket;

use crate::bucket_client_error::BucketClientError;
use crate::object_key::ObjectKey;

/// Arguments for [`BucketClient::upload_bytes`].
pub struct UploadBytesArgs<'a, S: AsRef<str>> {
  /// The destination object key (normalized via [`ObjectKey`]).
  pub object_key: S,
  /// The raw bytes to upload.
  pub bytes: &'a [u8],
  /// Optional MIME type. Defaults to `application/octet-stream` when `None`.
  pub content_type: Option<&'a str>,
}

/// A client for a single S3-compatible bucket. Cheap to clone. Build one with
/// [`crate::BucketClientBuilder`].
#[derive(Clone)]
pub struct BucketClient {
  bucket: Bucket,
}

/// Signed parts of a RestoreObject request. The caller sends the same body bytes.
pub struct SignedRestoreRequest {
  pub url: String,
  pub headers: Vec<(String, String)>,
}

impl BucketClient {
  /// Wrap a fully-configured `Bucket`. Internal — use [`crate::BucketClientBuilder`].
  pub(crate) fn from_bucket(bucket: Bucket) -> Self {
    Self { bucket }
  }

  /// The name of the bucket this client targets.
  pub fn bucket_name(&self) -> &str {
    self.bucket.name.as_str()
  }

  pub async fn presign_download(&self, object_key: &str, expires_in: u32) -> Result<String, BucketClientError> {
    let key = ObjectKey::new(object_key);
    Ok(self.bucket.presign_get(key.as_str(), expires_in, None).await?)
  }

  /// The PUT must send the signed `If-None-Match: *` precondition.
  pub async fn presign_create(&self, object_key: &str, expires_in: u32) -> Result<String, BucketClientError> {
    let key = ObjectKey::new(object_key);
    let mut headers = self.bucket.extra_headers().clone();
    headers.insert("if-none-match", "*".parse().expect("a valid precondition"));
    Ok(self.bucket.presign_put(key.as_str(), expires_in, Some(headers), None).await?)
  }

  pub async fn presign_delete(&self, object_key: &str, expires_in: u32) -> Result<String, BucketClientError> {
    let key = ObjectKey::new(object_key);
    Ok(self.bucket.presign_delete(key.as_str(), expires_in).await?)
  }

  pub async fn sign_restore(&self, object_key: &str, body: &[u8]) -> Result<SignedRestoreRequest, BucketClientError> {
    let key = ObjectKey::new(object_key);
    let request = ReqwestRequest {
      bucket: &self.bucket,
      path: key.as_str(),
      command: Command::PutObject { content: body, content_type: "application/xml", multipart: None },
      datetime: s3::utils::now_utc(),
      sync: false,
    };
    let mut url = request.url()?;
    url.set_query(Some("restore"));

    let mut headers = self.bucket.extra_headers().clone();
    let payload_sha = request.command.sha256()?;
    headers.insert("host", request.host_header().parse().map_err(S3Error::from)?);
    headers.insert("content-type", "application/xml".parse().expect("valid media type"));
    headers.insert("x-amz-date", request.long_date()?.parse().map_err(S3Error::from)?);
    headers.insert("x-amz-content-sha256", payload_sha.parse().map_err(S3Error::from)?);
    if let Some(token) = self.bucket.session_token().await? {
      headers.insert("x-amz-security-token", token.parse().map_err(S3Error::from)?);
    }
    let canonical = signing::canonical_request("POST", &url, &headers, &payload_sha)?;
    let string_to_sign = request.string_to_sign(&canonical)?;
    let identity = self.bucket.access_key().await?.ok_or_else(|| {
      BucketClientError::ClientBuilderSetupError("archive restoration requires a bucket access key".into())
    })?;
    let mut mac = signing::HmacSha256::new_from_slice(&request.signing_key().await?).map_err(S3Error::from)?;
    mac.update(string_to_sign.as_bytes());
    let authorization = signing::authorization_header(
      &identity, &request.datetime(), &self.bucket.region(),
      &signing::signed_header_string(&headers), &hex::encode(mac.finalize().into_bytes()),
    )?;
    headers.insert("authorization", authorization.parse().map_err(S3Error::from)?);
    let headers = headers.iter().map(|(name, value)| {
      Ok::<_, S3Error>((name.as_str().to_owned(), value.to_str()?.to_owned()))
    }).collect::<Result<_, _>>()?;
    Ok(SignedRestoreRequest { url: url.into(), headers })
  }

  /// Upload raw bytes to `object_key`, optionally with an explicit content type.
  pub async fn upload_bytes<S: AsRef<str>>(&self, args: UploadBytesArgs<'_, S>) -> Result<(), BucketClientError> {
    let key = ObjectKey::new(&args.object_key);

    let response = match args.content_type {
      Some(content_type) => self.bucket.put_object_with_content_type(key.as_str(), args.bytes, content_type).await?,
      None => self.bucket.put_object(key.as_str(), args.bytes).await?,
    };

    let status_code = response.status_code();
    if !(200..300).contains(&status_code) {
      return Err(BucketClientError::RequestFailed {
        status_code,
        message: String::from_utf8_lossy(response.bytes()).into_owned(),
      });
    }
    Ok(())
  }

  /// Download an object's bytes (and its content type, when the store reports one).
  pub async fn download_bytes<S: AsRef<str>>(&self, object_key: S) -> Result<(Vec<u8>, Option<String>), BucketClientError> {
    let key = ObjectKey::new(&object_key);
    let response = self.bucket.get_object(key.as_str()).await?;

    let status_code = response.status_code();
    if !(200..300).contains(&status_code) {
      return Err(BucketClientError::RequestFailed {
        status_code,
        message: String::from_utf8_lossy(response.bytes()).into_owned(),
      });
    }
    let content_type = response.headers().get("content-type").map(|v| v.to_string());
    Ok((response.to_vec(), content_type))
  }

  /// Delete an object. Deleting a nonexistent key is a success (S3 semantics).
  pub async fn delete_object<S: AsRef<str>>(&self, object_key: S) -> Result<(), BucketClientError> {
    let key = ObjectKey::new(&object_key);
    let response = self.bucket.delete_object(key.as_str()).await?;

    let status_code = response.status_code();
    if !(200..300).contains(&status_code) {
      return Err(BucketClientError::RequestFailed {
        status_code,
        message: String::from_utf8_lossy(response.bytes()).into_owned(),
      });
    }
    Ok(())
  }
}

// ── Live round-trip tests (ignored — need real credentials) ──
//
// Drop these files under `{repo}/external/credentials/` (gitignored):
//   r2_endpoint.txt           e.g. https://<account_id>.r2.cloudflarestorage.com
//   r2_access_key_id.txt
//   r2_secret_access_key.txt
//   r2_bucket_name.txt        e.g. foundation-dev
// then run:  cargo test -p bucket_client -- --ignored
#[cfg(test)]
mod live_tests {
  use super::*;
  use crate::BucketClientBuilder;

  fn read_credential(file: &str) -> anyhow::Result<String> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../../external/credentials")
      .join(file);
    let raw = std::fs::read_to_string(&path)
      .map_err(|e| anyhow::anyhow!("failed to read {} ({e}) — drop the value there for live tests", path.display()))?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
      anyhow::bail!("credential file {} is empty", path.display());
    }
    Ok(trimmed)
  }

  fn live_client() -> anyhow::Result<BucketClient> {
    BucketClientBuilder::new()
      .endpoint(read_credential("r2_endpoint.txt")?)
      .access_key(read_credential("r2_access_key_id.txt")?)
      .secret_key(read_credential("r2_secret_access_key.txt")?)
      .region_name("auto")
      .bucket_name(read_credential("r2_bucket_name.txt")?)
      .build()
      .map_err(|e| anyhow::anyhow!("{e}"))
  }

  /// A tiny valid 1×1 PNG (89 bytes) so the round-trip exercises real image bytes.
  const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
    0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
    0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0xF8,
    0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x6F, 0x1B, 0xB4, 0xC6, 0x00, 0x00, 0x00,
    0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
  ];

  #[tokio::test]
  #[ignore] // live: requires real R2 credentials
  async fn live_upload_download_delete_roundtrip() -> anyhow::Result<()> {
    let client = live_client()?;
    let key = format!("test/bucket-client-roundtrip-{}.png", std::process::id());

    client
      .upload_bytes(UploadBytesArgs { object_key: &key, bytes: TINY_PNG, content_type: Some("image/png") })
      .await
      .map_err(|e| anyhow::anyhow!("upload: {e}"))?;

    let (bytes, content_type) = client.download_bytes(&key).await.map_err(|e| anyhow::anyhow!("download: {e}"))?;
    assert_eq!(bytes, TINY_PNG, "downloaded bytes should match the upload");
    println!("round-trip ok: {} bytes, content-type {:?}", bytes.len(), content_type);

    client.delete_object(&key).await.map_err(|e| anyhow::anyhow!("delete: {e}"))?;

    // After deletion the object is gone (fail-on-err surfaces the 404 as an S3 error).
    assert!(client.download_bytes(&key).await.is_err(), "object should be gone after delete");
    Ok(())
  }

  #[tokio::test]
  #[ignore] // live: requires real R2 credentials
  async fn live_avatar_shaped_upload() -> anyhow::Result<()> {
    // Exercise the exact key shape the avatar pipeline uses (files.md §2, folder=avatars).
    let client = live_client()?;
    let hash = "7q3k9m2r5t8x1c4f7j0v2b5d8g1n4p6s";
    let key = format!(
      "avatars/{}/{}/{}/{}/{}/{}/{hash}_512.png",
      &hash[0..1], &hash[1..2], &hash[2..3], &hash[3..4], &hash[4..5], &hash[5..6]
    );

    client
      .upload_bytes(UploadBytesArgs { object_key: &key, bytes: TINY_PNG, content_type: Some("image/png") })
      .await
      .map_err(|e| anyhow::anyhow!("upload: {e}"))?;
    let (bytes, _) = client.download_bytes(&key).await.map_err(|e| anyhow::anyhow!("download: {e}"))?;
    assert_eq!(bytes, TINY_PNG);
    client.delete_object(&key).await.map_err(|e| anyhow::anyhow!("delete: {e}"))?;
    Ok(())
  }
}
