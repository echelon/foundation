use axum::http::{
    header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH, RETRY_AFTER},
    HeaderMap, HeaderValue, StatusCode,
};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use super::{catalog::digest, requests::RequestStatus};
use crate::error::{ApiError, ApiResult};

pub(super) fn status_response(
    request_headers: &HeaderMap,
    status: RequestStatus,
    value: &impl Serialize,
) -> ApiResult<Response> {
    // Hash the authorized view so status, cost and metadata edits all invalidate it.
    let body = serde_json::to_vec(value)
        .map_err(|_| ApiError::Internal("could not encode request status".into()))?;
    let tag = digest(&body);
    let validator = HeaderValue::from_str(&format!("W/\"{tag}\""))
        .map_err(|_| ApiError::Internal("could not encode request validator".into()))?;
    let mut response = if matches_validator(request_headers, tag.as_bytes()) {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        ([(CONTENT_TYPE, "application/json")], body).into_response()
    };
    let headers = response.headers_mut();
    headers.insert(ETAG, validator);
    if matches!(status, RequestStatus::Queued | RequestStatus::Running) {
        headers.insert(RETRY_AFTER, HeaderValue::from_static("1"));
    }
    Ok(response)
}

fn matches_validator(headers: &HeaderMap, tag: &[u8]) -> bool {
    let values = headers.get_all(IF_NONE_MATCH);
    let mut values = values.iter();
    let Some(first) = values.next() else {
        return false;
    };
    if first.as_bytes().trim_ascii() == b"*" {
        return values.next().is_none();
    }
    std::iter::once(first)
        .chain(values)
        .try_fold(false, |matched, value| {
            match_tag_list(value.as_bytes(), tag).map(|found| matched || found)
        })
        .unwrap_or(false)
}

fn match_tag_list(mut rest: &[u8], tag: &[u8]) -> Option<bool> {
    let mut matched = false;
    loop {
        rest = rest.trim_ascii_start();
        if rest.is_empty() {
            return Some(matched);
        }
        if let Some(next) = rest.strip_prefix(b",") {
            rest = next;
            continue;
        }
        rest = rest.strip_prefix(b"W/").unwrap_or(rest).strip_prefix(b"\"")?;
        let end = rest.iter().position(|&byte| byte == b'"')?;
        let opaque = &rest[..end];
        if !opaque
            .iter()
            .all(|&byte| byte == 0x21 || (0x23..=0x7e).contains(&byte) || byte >= 0x80)
        {
            return None;
        }
        matched |= opaque == tag;
        rest = rest[end + 1..].trim_ascii_start();
        if rest.is_empty() {
            return Some(matched);
        }
        rest = rest.strip_prefix(b",")?;
    }
}
