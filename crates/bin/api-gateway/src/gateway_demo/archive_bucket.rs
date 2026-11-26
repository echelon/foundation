use std::time::Duration;

use anyhow::{ensure, Context};
use bucket_client::{BucketClient, BucketClientBuilder, SignedObjectRequest};
use reqwest::header::{HeaderMap, CONTENT_TYPE, IF_NONE_MATCH};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};

use super::archive_blobs::{BAD_OBJECT, COLD_OBJECT, IO_ERROR};
use super::catalog::digest;

const SIGNATURE_SECONDS: u32 = 60;
const RESTORE_BODY: &str = concat!(
    r#"<RestoreRequest xmlns="http://s3.amazonaws.com/doc/2006-03-01/">"#,
    "<Days>1</Days><GlacierJobParameters><Tier>Standard</Tier></GlacierJobParameters></RestoreRequest>",
);

pub(super) struct ArchiveBucket {
    endpoint: Url,
    prefix: String,
    target: String,
    auto_restore: bool,
    signer: BucketClient,
    http: Client,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RestoreDispatch {
    Confirmed,
    Unconfirmed,
}

impl ArchiveBucket {
    pub(super) fn load(
        namespace: &str,
        default_prefix: &str,
        auto_restore_setting: Option<&str>,
    ) -> anyhow::Result<Option<Self>> {
        let [bucket, region, endpoint, access_key, secret_key, session_token, prefix] =
            ["BUCKET", "REGION", "ENDPOINT", "ACCESS_KEY", "SECRET_KEY", "SESSION_TOKEN", "PREFIX"]
                .map(|suffix| format!("{namespace}_{suffix}"));
        if ![
            &bucket, &region, &endpoint, &access_key, &secret_key, &session_token, &prefix,
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
            && !auto_restore_setting.is_some_and(|name| std::env::var_os(name).is_some())
        {
            return Ok(None);
        }
        let bucket = setting(&bucket)?;
        let region = setting(&region)?;
        let auto_restore = if let Some(name) = auto_restore_setting {
            match std::env::var(name).as_deref() {
                Err(std::env::VarError::NotPresent) | Ok("0") => false,
                Ok("1") => true,
                _ => anyhow::bail!("set {name} to 0 or 1"),
            }
        } else {
            false
        };
        let endpoint = Url::parse(&setting(&endpoint)?)
            .map_err(|_| anyhow::anyhow!("invalid archive S3 endpoint"))?;
        let local = endpoint.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        ensure!(
            endpoint.host().is_some()
                && (endpoint.scheme() == "https" || (local && endpoint.scheme() == "http"))
                && endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.path() == "/"
                && endpoint.query().is_none()
                && endpoint.fragment().is_none(),
            "archive S3 endpoint must be an HTTPS origin or a literal loopback HTTP origin"
        );
        ensure!(
            (3..=63).contains(&bucket.len())
                && bucket.split('.').all(|segment| {
                    !segment.is_empty()
                        && segment.as_bytes()[0].is_ascii_alphanumeric()
                        && segment.as_bytes()[segment.len() - 1].is_ascii_alphanumeric()
                        && segment.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                        })
                }),
            "invalid archive S3 bucket name"
        );
        ensure!(
            region.len() <= 64
                && region
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'),
            "invalid archive S3 signing region"
        );
        let prefix = match std::env::var(&prefix) {
            Ok(prefix) => prefix,
            Err(std::env::VarError::NotPresent) => default_prefix.into(),
            Err(_) => anyhow::bail!("archive S3 prefix must be UTF-8"),
        };
        ensure!(
            prefix.len() <= 120
                && prefix.split('/').all(|part| !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))),
            "archive S3 prefix must use nonempty ASCII path segments"
        );
        let target = format!(
            "s3_{}",
            digest(serde_json::to_vec(&(
                endpoint.as_str(),
                &region,
                &bucket,
                &prefix
            ))?)
        );
        let mut signer = BucketClientBuilder::new()
            .endpoint(endpoint.as_str().trim_end_matches('/'))
            .region_name(region)
            .bucket_name(bucket)
            .access_key(setting(&access_key)?)
            .secret_key(setting(&secret_key)?);
        if std::env::var_os(&session_token).is_some() {
            signer = signer.session_token(setting(&session_token)?);
        }
        let signer = signer
            .build()
            .map_err(|_| anyhow::anyhow!("could not configure the archive bucket"))?;
        let http = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Some(Self {
            endpoint,
            prefix,
            target,
            auto_restore,
            signer,
            http,
        }))
    }

    pub(super) fn target(&self) -> &str {
        &self.target
    }

    pub(super) fn permits_restore(&self) -> bool {
        self.auto_restore
    }

    fn key(&self, name: &str) -> String {
        format!("{}/{name}", self.prefix)
    }

    fn signed_url(
        &self,
        signed: Result<String, bucket_client::BucketClientError>,
    ) -> Result<Url, &'static str> {
        let url = Url::parse(&signed.map_err(|_| IO_ERROR)?).map_err(|_| IO_ERROR)?;
        if url.origin() != self.endpoint.origin() {
            return Err(IO_ERROR);
        }
        Ok(url)
    }

    pub(super) async fn put(&self, name: &str, bytes: Vec<u8>) -> Result<(), &'static str> {
        let url = self.signed_url(
            self.signer
                .presign_create(&self.key(name), SIGNATURE_SECONDS)
                .await,
        )?;
        let result = self
            .http
            .put(url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .header(IF_NONE_MATCH, "*")
            .body(bytes)
            .send()
            .await
            .map_err(|_| IO_ERROR)?;
        if result.status().is_success() {
            Ok(())
        } else {
            Err(IO_ERROR)
        }
    }

    pub(super) async fn get(&self, name: &str, limit: usize) -> Result<Vec<u8>, &'static str> {
        let url = self.signed_url(
            self.signer
                .presign_download(&self.key(name), SIGNATURE_SECONDS)
                .await,
        )?;
        let result = self.http.get(url).send().await.map_err(|_| IO_ERROR)?;
        match result.status() {
            StatusCode::OK => read_bounded(result, limit).await,
            StatusCode::FORBIDDEN => {
                let cold = has_error(result, "<Code>InvalidObjectState</Code>").await;
                Err(if cold { COLD_OBJECT } else { IO_ERROR })
            }
            _ => Err(IO_ERROR),
        }
    }

    pub(super) async fn restoration_exists(&self, name: &str) -> Result<bool, &'static str> {
        let signed = self.signer.sign_head(&self.key(name)).await.map_err(|_| IO_ERROR)?;
        let response = self.signed_request(Method::HEAD, signed)?.send().await.map_err(|_| IO_ERROR)?;
        if response.status() != StatusCode::OK {
            return Err(IO_ERROR);
        }
        restoration_exists(response.headers())
    }

    pub(super) async fn request_restore(&self, name: &str) -> Result<RestoreDispatch, &'static str> {
        if !self.auto_restore {
            return Err(COLD_OBJECT);
        }
        let signed = self
            .signer
            .sign_restore(&self.key(name), RESTORE_BODY.as_bytes())
            .await
            .map_err(|_| IO_ERROR)?;
        let Ok(result) = self.signed_request(Method::POST, signed)?.body(RESTORE_BODY).send().await else {
            return Ok(RestoreDispatch::Unconfirmed);
        };
        let status = result.status();
        if status.is_server_error() || matches!(status, StatusCode::TOO_MANY_REQUESTS | StatusCode::REQUEST_TIMEOUT) {
            return Ok(RestoreDispatch::Unconfirmed);
        }
        if matches!(status, StatusCode::OK | StatusCode::ACCEPTED)
            || (status == StatusCode::CONFLICT
                && has_error(result, "<Code>RestoreAlreadyInProgress</Code>").await)
        {
            return Ok(RestoreDispatch::Confirmed);
        }
        Err("archive_retrieval_rejected")
    }

    fn signed_request(&self, method: Method, signed: SignedObjectRequest) -> Result<RequestBuilder, &'static str> {
        let url = self.signed_url(Ok(signed.url))?;
        let mut request = self.http.request(method, url);
        for (name, value) in signed.headers {
            let mut value = reqwest::header::HeaderValue::from_str(&value).map_err(|_| IO_ERROR)?;
            value.set_sensitive(true);
            request = request.header(name, value);
        }
        Ok(request)
    }

    pub(super) async fn delete(&self, name: &str) -> Result<(), &'static str> {
        let url = self.signed_url(
            self.signer
                .presign_delete(&self.key(name), SIGNATURE_SECONDS)
                .await,
        )?;
        let result = self.http.delete(url).send().await.map_err(|_| IO_ERROR)?;
        // A delete marker hides the object but leaves its previous versions in storage.
        if !result.status().is_success()
            || result
                .headers()
                .get("x-amz-delete-marker")
                .is_some_and(|value| value == "true")
            || result
                .headers()
                .get("x-amz-version-id")
                .is_some_and(|value| value != "null")
        {
            return Err(IO_ERROR);
        }
        Ok(())
    }
}

fn restoration_exists(headers: &HeaderMap) -> Result<bool, &'static str> {
    let mut fields = headers.get_all("x-amz-restore").iter();
    let Some(field) = fields.next() else { return Ok(false) };
    if fields.next().is_some() {
        return Err(IO_ERROR);
    }
    let status = field.to_str().map_err(|_| IO_ERROR)?.split(',').next().map(str::trim);
    match status {
        Some("ongoing-request=\"true\"" | "ongoing-request=\"false\"") => Ok(true),
        _ => Err(IO_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn missing_restore_metadata_allows_recovery_and_invalid_metadata_defers_it() {
        let mut headers = HeaderMap::new();
        assert_eq!(restoration_exists(&headers), Ok(false));
        for value in [
            "ongoing-request=\"true\"",
            "ongoing-request=\"false\", expiry-date=\"Fri, 21 Dec 2012 00:00:00 GMT\"",
        ] {
            headers.insert("x-amz-restore", HeaderValue::from_static(value));
            assert_eq!(restoration_exists(&headers), Ok(true));
        }
        headers.insert("x-amz-restore", HeaderValue::from_static("unknown"));
        assert_eq!(restoration_exists(&headers), Err(IO_ERROR));
        headers.append("x-amz-restore", HeaderValue::from_static("ongoing-request=\"true\""));
        assert_eq!(restoration_exists(&headers), Err(IO_ERROR));
    }
}

async fn has_error(response: Response, tag: &str) -> bool {
    read_bounded(response, 4096)
        .await
        .ok()
        .is_some_and(|body| String::from_utf8_lossy(&body).contains(tag))
}

fn setting(name: &str) -> anyhow::Result<String> {
    let value =
        std::env::var(name).with_context(|| format!("set {name} for archive S3 storage"))?;
    ensure!(
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic()),
        "invalid {name}"
    );
    Ok(value)
}

async fn read_bounded(mut response: Response, max: usize) -> Result<Vec<u8>, &'static str> {
    if response
        .content_length()
        .is_some_and(|length| length > max as u64)
    {
        return Err(BAD_OBJECT);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| IO_ERROR)? {
        if chunk.len() > max - body.len() {
            return Err(BAD_OBJECT);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
