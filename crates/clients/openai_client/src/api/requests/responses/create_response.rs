//! The shared create-side transport: `POST /v1/responses` with `background: true`. Every
//! model binding funnels through here; none of them ever hold a generation open.

use tracing::info;

use crate::api::requests::openai_host::{with_auth, OPENAI_API_BASE_URL};
use crate::api::requests::responses::wire::{CreateResponseBody, ResponseWire};
use crate::api::types::response_types::response_id::ResponseId;
use crate::api::types::response_types::response_status::ResponseStatus;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::classify_openai_http_error::classify_openai_http_error;
use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;

/// What every start binding returns: the handle to poll. The heavy lifting happens on
/// OpenAI's side; retrieve progress/results with `get_response` / `await_response`.
#[derive(Clone, Debug)]
pub struct StartedResponse {
  pub response_id: ResponseId,
  /// Usually `Queued` at creation time.
  pub status: ResponseStatus,
}

pub(crate) async fn create_background_response(
  api_key: &OpenAiApiKey,
  body: &CreateResponseBody,
) -> Result<StartedResponse, OpenAiError> {
  info!(model = %body.model, "openai create background response");

  let client = client_core::http_client("foundation/openai-client");
  let builder = client
    .post(format!("{OPENAI_API_BASE_URL}/v1/responses"))
    .header("Content-Type", "application/json")
    .json(body);

  let response = with_auth(builder, api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, "openai create response reply");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: ResponseWire = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;

  Ok(StartedResponse {
    response_id: ResponseId(parsed.id),
    status: ResponseStatus::from_wire(&parsed.status),
  })
}
