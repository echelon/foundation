//! Shared image-start plumbing. Generation and EDIT are the SAME call on the Responses
//! API: the `image_generation` tool renders from the message — a bare prompt is
//! text-to-image; prompt + input images is the edit/composite modality. Each model binding
//! builds one of these and every request runs `background: true` + poll.

use crate::api::requests::responses::create_response::{create_background_response, StartedResponse};
use crate::api::requests::responses::wire::{
  ContentPart, CreateResponseBody, ForcedToolChoice, ImageGenerationTool, InputMessage, MaskReference,
  MessageContent,
};
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::api::types::chat_types::chat_role::ChatRole;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;
use crate::api::types::image_types::image_size::ImageSize;
use crate::api::types::image_types::input_fidelity::InputFidelity;
use crate::api::types::image_types::input_image_source::InputImageSource;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;

/// The Responses API needs a "driver" LLM to invoke the image tool; the tool's `model`
/// field is what picks the GPT Image model. The driver's token cost is a rounding error
/// next to the image tokens — we default to the cheapest tier.
pub(crate) const DEFAULT_IMAGE_DRIVER: ChatModel = ChatModel::Gpt5_4Nano;

pub(crate) struct ImageStart<'a> {
  pub model: ImageModel,
  pub prompt: &'a str,
  pub input_images: &'a [InputImageSource],
  pub mask: Option<&'a InputImageSource>,
  pub size: Option<ImageSize>,
  pub quality: Option<ImageQuality>,
  pub output_format: Option<OutputFormat>,
  pub input_fidelity: Option<InputFidelity>,
  pub driver_model: Option<&'a ChatModel>,
}

pub(crate) fn build_image_body(start: &ImageStart<'_>) -> Result<CreateResponseBody, OpenAiClientError> {
  if start.prompt.trim().is_empty() {
    return Err(OpenAiClientError::InvalidRequest("prompt must not be empty".into()));
  }
  if start.mask.is_some() && start.input_images.is_empty() {
    return Err(OpenAiClientError::InvalidRequest("a mask requires at least one input image".into()));
  }

  let mut parts: Vec<ContentPart> = vec![ContentPart::Text { text: start.prompt.to_string() }];
  for image in start.input_images {
    let (image_url, file_id) = image.to_wire();
    parts.push(ContentPart::Image { image_url, file_id });
  }

  let mask = start.mask.map(|m| {
    let (image_url, file_id) = m.to_wire();
    MaskReference { image_url, file_id }
  });

  let driver = start.driver_model.unwrap_or(&DEFAULT_IMAGE_DRIVER);
  Ok(CreateResponseBody {
    model: driver.as_str().to_string(),
    input: vec![InputMessage { role: ChatRole::User, content: MessageContent::Parts(parts) }],
    background: true,
    store: true,
    max_output_tokens: None,
    temperature: None,
    top_p: None,
    user: None,
    tools: Some(vec![ImageGenerationTool {
      kind: "image_generation",
      model: start.model.as_str().to_string(),
      size: start.size,
      quality: start.quality,
      output_format: start.output_format,
      input_fidelity: start.input_fidelity,
      input_image_mask: mask,
    }]),
    tool_choice: Some(ForcedToolChoice { kind: "image_generation" }),
  })
}

pub(crate) async fn start_image(
  api_key: &OpenAiApiKey,
  start: &ImageStart<'_>,
) -> Result<StartedResponse, OpenAiError> {
  let body = build_image_body(start)?;
  create_background_response(api_key, &body).await
}
