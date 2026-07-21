//! `generate_gpt_image_2` — GPT Image 2 via `/v1/responses` (`background: true`, then poll with
//! `get_response` / `await_response`). Text-to-image AND edit through the one interface:
//! leave `input_images` empty to generate; supply images (± mask) to edit/composite.
//! Input images are PUBLIC URLS (preferred — ours, CDN-managed) or uploaded file ids
//! (`upload_image_file`); never inline base64.

use crate::api::requests::images::common::{start_image, ImageStart};
use crate::api::requests::responses::create_response::StartedResponse;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;
use crate::api::types::image_types::image_size::ImageSize;
use crate::api::types::image_types::input_image_source::InputImageSource;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_error::OpenAiError;

// ── Public args ──

#[derive(Debug)]
pub struct GenerateGptImage2Args<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: GptImage2Request,
}

/// One 2 render. The model is pinned by this binding — no model field to get wrong.
#[derive(Clone, Debug)]
pub struct GptImage2Request {
  pub prompt: String,
  /// Empty = text-to-image. Non-empty = the EDIT modality: the model renders from these
  /// (public URLs preferred) plus the prompt.
  pub input_images: Vec<InputImageSource>,
  /// Optional mask constraining the editable region (requires at least one input image).
  pub mask: Option<InputImageSource>,
  pub size: Option<ImageSize>,
  pub quality: Option<ImageQuality>,
  pub output_format: Option<OutputFormat>,
  // NOTE: `input_fidelity` deliberately absent — gpt-image-2 does not support it.
  /// The LLM that drives the tool call (defaults to the cheapest tier; the image model
  /// itself is fixed to gpt-image-2).
  pub driver_model: Option<ChatModel>,
}

impl GptImage2Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      input_images: Vec::new(),
      mask: None,
      size: None,
      quality: None,
      output_format: None,
      driver_model: None,
    }
  }
}

pub(crate) fn to_start(request: &GptImage2Request) -> ImageStart<'_> {
  ImageStart {
    model: ImageModel::GptImage2,
    prompt: &request.prompt,
    input_images: &request.input_images,
    mask: request.mask.as_ref(),
    size: request.size,
    quality: request.quality,
    output_format: request.output_format,
    input_fidelity: None,
    driver_model: request.driver_model.as_ref(),
  }
}

/// Start a 2 generation/edit in the background. Returns immediately with the
/// response id — nothing is held open. Poll with `get_response`, or `await_response`.
pub async fn generate_gpt_image_2(args: GenerateGptImage2Args<'_>) -> Result<StartedResponse, OpenAiError> {
  start_image(args.api_key, &to_start(&args.request)).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::images::common::build_image_body;
  use crate::error::openai_client_error::OpenAiClientError;

  /// Input struct → exact outbound HTTP body shape.
  #[test]
  fn wire_shape_text_to_image() {
    let mut req = GptImage2Request::new("a red fox");
    req.size = Some(ImageSize::Square1024);
    req.quality = Some(ImageQuality::Low);
    let body = serde_json::to_value(build_image_body(&to_start(&req)).unwrap()).unwrap();
    assert_eq!(body["background"], true);
    assert_eq!(body["store"], true);
    assert_eq!(body["model"], "gpt-5.4-nano"); // default driver
    assert_eq!(body["tool_choice"]["type"], "image_generation");
    let tool = &body["tools"][0];
    assert_eq!(tool["type"], "image_generation");
    assert_eq!(tool["model"], "gpt-image-2");
    assert_eq!(tool["size"], "1024x1024");
    assert_eq!(tool["quality"], "low");
    assert!(tool.get("input_image_mask").is_none());
    let content = &body["input"][0]["content"];
    assert_eq!(content[0]["type"], "input_text");
    assert_eq!(content[0]["text"], "a red fox");
    assert_eq!(content.as_array().unwrap().len(), 1);
  }

  /// The EDIT modality: input images ride in the message (URL / file id — never base64);
  /// the mask in the tool config.
  #[test]
  fn wire_shape_edit_with_mask() {
    let mut req = GptImage2Request::new("make it night");
    req.input_images = vec![
      InputImageSource::url("https://cdn.test/room.png"),
      InputImageSource::file_id("file-9"),
    ];
    req.mask = Some(InputImageSource::url("https://cdn.test/mask.png"));
    let body = serde_json::to_value(build_image_body(&to_start(&req)).unwrap()).unwrap();
    let content = &body["input"][0]["content"];
    assert_eq!(content[1]["type"], "input_image");
    assert_eq!(content[1]["image_url"], "https://cdn.test/room.png");
    assert!(content[1].get("file_id").is_none());
    assert_eq!(content[2]["file_id"], "file-9");
    assert!(content[2].get("image_url").is_none());
    assert_eq!(body["tools"][0]["input_image_mask"]["image_url"], "https://cdn.test/mask.png");
  }

  #[test]
  fn empty_prompt_and_orphan_mask_rejected() {
    let empty = GptImage2Request::new("   ");
    assert!(matches!(
      build_image_body(&to_start(&empty)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
    let mut orphan = GptImage2Request::new("x");
    orphan.mask = Some(InputImageSource::url("https://cdn.test/mask.png"));
    assert!(matches!(
      build_image_body(&to_start(&orphan)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
  }

  #[tokio::test]
  #[ignore] // live — costs money; needs external/credentials/openai_api_key.txt
  async fn live_generate_gpt_image_2() -> anyhow::Result<()> {
    use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
    use crate::api::types::response_types::response_status::ResponseStatus;
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let mut req = GptImage2Request::new("a tiny pixel-art rocket, plain background");
    req.quality = Some(ImageQuality::Low);
    req.size = Some(ImageSize::Square1024);
    let started = generate_gpt_image_2(GenerateGptImage2Args { api_key: &key, request: req }).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let done = await_response(
      AwaitResponseArgs { api_key: &key, response_id: &started.response_id },
      PollConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(done.status == ResponseStatus::Completed, "status: {:?} ({:?})", done.status, done.failure);
    anyhow::ensure!(!done.images.is_empty() && !done.images[0].bytes.is_empty());
    Ok(())
  }
}
