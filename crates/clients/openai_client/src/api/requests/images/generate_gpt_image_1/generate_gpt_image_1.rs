//! `generate_gpt_image_1` — GPT Image 1 via `/v1/responses` (`background: true`, then poll with
//! `get_response` / `await_response`). Text-to-image AND edit through the one interface:
//! leave `input_images` empty to generate; supply images (± mask) to edit/composite.
//! Input images are PUBLIC URLS (preferred — ours, CDN-managed) or uploaded file ids
//! (`upload_image_file`); never inline base64.

use crate::api::requests::images::common::{image_wire_json, start_image, ImageStart};
use crate::api::requests::responses::create_response::StartedResponse;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::api::types::image_types::image_action::ImageAction;
use crate::api::types::image_types::image_background::ImageBackground;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_moderation::ImageModeration;
use crate::api::types::image_types::image_quality::ImageQuality;
use crate::api::types::image_types::image_size::ImageSize;
use crate::api::types::image_types::input_fidelity::InputFidelity;
use crate::api::types::image_types::input_image_source::InputImageSource;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_error::OpenAiError;

// ── Public args ──

#[derive(Debug)]
pub struct GenerateGptImage1Args<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: GptImage1Request,
}

/// One 1 render. The model is pinned by this binding — no model field to get wrong.
#[derive(Clone, Debug)]
pub struct GptImage1Request {
  pub prompt: String,
  /// Empty = text-to-image. Non-empty = the EDIT modality: the model renders from these
  /// (public URLs preferred) plus the prompt.
  pub input_images: Vec<InputImageSource>,
  /// Optional mask constraining the editable region (requires at least one input image).
  pub mask: Option<InputImageSource>,
  /// Output dimensions (`auto` lets the model choose).
  pub size: Option<ImageSize>,
  /// Rendering quality tier — drives the price.
  pub quality: Option<ImageQuality>,
  /// Encoding of the returned image (`png` default).
  pub output_format: Option<OutputFormat>,
  /// Edit fidelity (how closely input images are preserved). Not on GPT Image 2.
  pub input_fidelity: Option<InputFidelity>,
  /// Force `generate`/`edit` instead of the server's `auto` inference.
  pub action: Option<ImageAction>,
  /// Output transparency (`transparent` needs png/webp output).
  pub background: Option<ImageBackground>,
  /// Content-filter strictness (`auto` default, `low` = less restrictive).
  pub moderation: Option<ImageModeration>,
  /// 0–100 compression for jpeg/webp outputs (validated client-side).
  pub output_compression: Option<u8>,
  /// The LLM that drives the tool call (defaults to the cheapest tier; the image model
  /// itself is fixed to gpt-image-1).
  pub driver_model: Option<ChatModel>,
}

impl GptImage1Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      input_images: Vec::new(),
      mask: None,
      size: None,
      quality: None,
      output_format: None,
      input_fidelity: None,
      action: None,
      background: None,
      moderation: None,
      output_compression: None,
      driver_model: None,
    }
  }
}

pub(crate) fn to_start(request: &GptImage1Request) -> ImageStart<'_> {
  ImageStart {
    model: ImageModel::GptImage1,
    prompt: &request.prompt,
    input_images: &request.input_images,
    mask: request.mask.as_ref(),
    size: request.size,
    quality: request.quality,
    output_format: request.output_format,
    input_fidelity: request.input_fidelity,
    action: request.action,
    background: request.background,
    moderation: request.moderation,
    output_compression: request.output_compression,
    driver_model: request.driver_model.as_ref(),
  }
}

/// Start a 1 generation/edit in the background. Returns immediately with the
/// response id — nothing is held open. Poll with `get_response`, or `await_response`.
pub async fn generate_gpt_image_1(args: GenerateGptImage1Args<'_>) -> Result<StartedResponse, OpenAiError> {
  start_image(args.api_key, &to_start(&args.request)).await
}


/// The exact `/v1/responses` JSON body this request would send — for the outbound
/// provider-call log (and dry runs, which log without sending).
pub fn generate_gpt_image_1_wire_json(request: &GptImage1Request) -> Result<serde_json::Value, OpenAiError> {
  image_wire_json(&to_start(request))
}

#[cfg(test)]
mod tests {

  /// New tool knobs → exact wire fields on the image_generation tool.
  #[test]
  fn wire_shape_with_action_background_moderation_compression() {
    use crate::api::requests::images::common::image_wire_json;
    use crate::api::types::image_types::output_format::OutputFormat;

    let mut req = GptImage1Request::new("a fox");
    req.action = Some(ImageAction::Generate);
    req.background = Some(ImageBackground::Transparent);
    req.moderation = Some(ImageModeration::Low);
    req.output_format = Some(OutputFormat::Webp);
    req.output_compression = Some(80);
    let body = image_wire_json(&super::to_start(&req)).unwrap();
    let tool = &body["tools"][0];
    assert_eq!(tool["action"], "generate");
    assert_eq!(tool["background"], "transparent");
    assert_eq!(tool["moderation"], "low");
    assert_eq!(tool["output_format"], "webp");
    assert_eq!(tool["output_compression"], 80);
  }

  /// output_compression is jpeg/webp-only and 0–100 — both rejected client-side.
  #[test]
  fn compression_validation() {
    use crate::api::requests::images::common::image_wire_json;
    use crate::api::types::image_types::output_format::OutputFormat;

    let mut png = GptImage1Request::new("a fox");
    png.output_compression = Some(50); // png default output
    assert!(image_wire_json(&super::to_start(&png)).is_err());

    let mut over = GptImage1Request::new("a fox");
    over.output_format = Some(OutputFormat::Jpeg);
    over.output_compression = Some(101);
    assert!(image_wire_json(&super::to_start(&over)).is_err());
  }
  use super::*;
  use crate::api::requests::images::common::build_image_body;
  use crate::error::openai_client_error::OpenAiClientError;

  /// Input struct → exact outbound HTTP body shape.
  #[test]
  fn wire_shape_text_to_image() {
    let mut req = GptImage1Request::new("a red fox");
    req.size = Some(ImageSize::Square1024);
    req.quality = Some(ImageQuality::Low);
    let body = serde_json::to_value(build_image_body(&to_start(&req)).unwrap()).unwrap();
    assert_eq!(body["background"], true);
    assert_eq!(body["store"], true);
    assert_eq!(body["model"], "gpt-5.4-nano"); // default driver
    assert_eq!(body["tool_choice"]["type"], "image_generation");
    let tool = &body["tools"][0];
    assert_eq!(tool["type"], "image_generation");
    assert_eq!(tool["model"], "gpt-image-1");
    assert_eq!(tool["size"], "1024x1024");
    assert_eq!(tool["quality"], "low");
    assert!(tool.get("input_image_mask").is_none());
    let content = &body["input"][0]["content"];
    assert_eq!(content[0]["type"], "input_text");
    assert_eq!(content[0]["text"], "a red fox");
    assert_eq!(content.as_array().unwrap().len(), 1);
    assert!(body.get("metadata").is_none()); // correlation is by response id, not metadata
  }

  /// The EDIT modality: input images ride in the message (URL / file id — never base64);
  /// the mask in the tool config. URLs are REAL stable test assets (`test_data` crate).
  #[test]
  fn wire_shape_edit_with_mask() {
    use test_data::web::image_urls::{
      TALL_MOCHI_WITH_GLASSES_GLASSES_MASK_IMAGE_URL, TALL_MOCHI_WITH_GLASSES_IMAGE_URL,
    };
    let mut req = GptImage1Request::new("make it night");
    req.input_images = vec![
      InputImageSource::url(TALL_MOCHI_WITH_GLASSES_IMAGE_URL),
      InputImageSource::file_id("file-9"),
    ];
    req.mask = Some(InputImageSource::url(TALL_MOCHI_WITH_GLASSES_GLASSES_MASK_IMAGE_URL));
    let body = serde_json::to_value(build_image_body(&to_start(&req)).unwrap()).unwrap();
    let content = &body["input"][0]["content"];
    assert_eq!(content[1]["type"], "input_image");
    assert_eq!(content[1]["image_url"], TALL_MOCHI_WITH_GLASSES_IMAGE_URL);
    assert!(content[1].get("file_id").is_none());
    assert_eq!(content[2]["file_id"], "file-9");
    assert!(content[2].get("image_url").is_none());
    assert_eq!(
      body["tools"][0]["input_image_mask"]["image_url"],
      TALL_MOCHI_WITH_GLASSES_GLASSES_MASK_IMAGE_URL
    );
  }

  #[test]
  fn empty_prompt_and_orphan_mask_rejected() {
    let empty = GptImage1Request::new("   ");
    assert!(matches!(
      build_image_body(&to_start(&empty)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
    let mut orphan = GptImage1Request::new("x");
    orphan.mask = Some(InputImageSource::url(test_data::web::image_urls::GHOST_IMAGE_URL));
    assert!(matches!(
      build_image_body(&to_start(&orphan)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
  }

  #[test]
  fn input_fidelity_serializes_into_the_tool() {
    let mut req = GptImage1Request::new("keep the face");
    req.input_images = vec![InputImageSource::file_id("file-1")];
    req.input_fidelity = Some(InputFidelity::High);
    let body = serde_json::to_value(build_image_body(&to_start(&req)).unwrap()).unwrap();
    assert_eq!(body["tools"][0]["input_fidelity"], "high");
  }

  // ── Live tests — cost money. Run with:
  //    cargo test -p openai_client generate_gpt_image_1 -- --ignored --nocapture
  // OpenAI's image tool returns BYTES (no hosted URL), so results are saved to the OS temp
  // dir and their file:// URLs printed before each test ends.

  async fn run_live(req: GptImage1Request, label: &str) -> anyhow::Result<Vec<String>> {
    use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
    use crate::api::types::response_types::response_status::ResponseStatus;
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::save_result_image::save_result_image;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let started = generate_gpt_image_1(GenerateGptImage1Args { api_key: &key, request: req }).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let done = await_response(
      AwaitResponseArgs { api_key: &key, response_id: &started.response_id },
      PollConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(done.status == ResponseStatus::Completed, "status: {:?} ({:?})", done.status, done.failure);
    anyhow::ensure!(!done.images.is_empty(), "no images in completed response");
    let mut urls = Vec::new();
    for (i, image) in done.images.iter().enumerate() {
      anyhow::ensure!(!image.bytes.is_empty(), "image {i} is empty");
      urls.push(save_result_image(image, &format!("{label}_{i}"))?);
    }
    println!("\n── gpt-image-1 result URL(s):");
    for url in &urls {
      println!("   {url}");
    }
    Ok(urls)
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_gpt_image_1() -> anyhow::Result<()> {
    let mut req = GptImage1Request::new("a tiny pixel-art rocket, plain background");
    req.quality = Some(ImageQuality::Low);
    req.size = Some(ImageSize::Square1024);
    run_live(req, "generate_gpt_image_1").await.map(|_| ())
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_gpt_image_1_edit_from_url() -> anyhow::Result<()> {
    // The EDIT modality end to end, from a REAL public test image URL (`test_data`).
    let mut req = GptImage1Request::new("Add a small wooden cabin nestled among the trees.");
    req.input_images = vec![InputImageSource::url(test_data::web::image_urls::FOREST_BACKDROP_IMAGE_URL)];
    req.quality = Some(ImageQuality::Low);
    req.size = Some(ImageSize::Square1024);
    run_live(req, "generate_gpt_image_1_edit").await.map(|_| ())
  }
}
