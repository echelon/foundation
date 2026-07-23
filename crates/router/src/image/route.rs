//! The built route: per-model state enums holding the fully-materialized provider-native
//! requests — one request PER SLICE (the fan-out already applied). artcraft_router shape,
//! minus dynamic providers.

use openai_client::api::requests::images::generate_gpt_image_1::generate_gpt_image_1::{
  generate_gpt_image_1, generate_gpt_image_1_wire_json, GenerateGptImage1Args, GptImage1Request,
};
use openai_client::api::requests::images::generate_gpt_image_1_5::generate_gpt_image_1_5::{
  generate_gpt_image_1_5, generate_gpt_image_1_5_wire_json, GenerateGptImage1_5Args, GptImage1_5Request,
};
use openai_client::api::requests::images::generate_gpt_image_1_mini::generate_gpt_image_1_mini::{
  generate_gpt_image_1_mini, generate_gpt_image_1_mini_wire_json, GenerateGptImage1MiniArgs, GptImage1MiniRequest,
};
use openai_client::api::requests::images::generate_gpt_image_2::generate_gpt_image_2::{
  generate_gpt_image_2, generate_gpt_image_2_wire_json, GenerateGptImage2Args, GptImage2Request,
};
use openai_client::creds::openai_api_key::OpenAiApiKey;

use google_gemini_api_client::api::requests::images::generate_nano_banana::generate_nano_banana::{
  generate_nano_banana_wire_json, NanoBananaRequest,
};
use google_gemini_api_client::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::{
  generate_nano_banana_2_wire_json, NanoBanana2Request,
};
use google_gemini_api_client::api::requests::images::generate_nano_banana_2_lite::generate_nano_banana_2_lite::{
  generate_nano_banana_2_lite_wire_json, NanoBanana2LiteRequest,
};
use google_gemini_api_client::api::requests::images::generate_nano_banana_pro::generate_nano_banana_pro::{
  generate_nano_banana_pro_wire_json, NanoBananaProRequest,
};

use google_gemini_api_client::api::requests::images::generate_nano_banana::generate_nano_banana::generate_nano_banana;
use google_gemini_api_client::api::requests::images::generate_nano_banana::generate_nano_banana::GenerateNanoBananaArgs;
use google_gemini_api_client::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::{
  generate_nano_banana_2, GenerateNanoBanana2Args,
};
use google_gemini_api_client::api::requests::images::generate_nano_banana_2_lite::generate_nano_banana_2_lite::{
  generate_nano_banana_2_lite, GenerateNanoBanana2LiteArgs,
};
use google_gemini_api_client::api::requests::images::generate_nano_banana_pro::generate_nano_banana_pro::{
  generate_nano_banana_pro, GenerateNanoBananaProArgs,
};
use google_gemini_api_client::api::types::image_types::image_input::ImageInput;
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

use crate::image::poll::RoutedImage;
use crate::input_strategy::{input_strategy_for, InputStrategy};

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::plan::RouterImageModel;
use crate::pricing::{PriceCertainty, PriceLineItem, PriceQuote};

/// A provider-side handle for ONE slice (for OpenAI: the background response id).
#[derive(Clone, Debug)]
pub struct ProviderHandle {
  pub provider_job_id: String,
}

/// A Google (upload-first) route's slices: N provider-native requests PLUS the
/// input CDN urls that dispatch must upload to the provider's Files API and
/// inject before each call (Gemini cannot fetch public URLs).
#[derive(Debug)]
pub struct GoogleImageSlices<R> {
  pub requests: Vec<R>,
  pub input_urls: Vec<String>,
}

/// One Google image slice, typed — what the outbound dispatcher matches on.
#[derive(Debug)]
pub enum GoogleImageSliceRef<'a> {
  NanoBanana(&'a NanoBananaRequest),
  NanoBananaPro(&'a NanoBananaProRequest),
  NanoBanana2(&'a NanoBanana2Request),
  NanoBanana2Lite(&'a NanoBanana2LiteRequest),
}

/// The built, ready-to-dispatch route. One variant per catalog model (hard-coded).
#[derive(Debug)]
pub enum ImageRoute {
  OpenaiGptImage1(Vec<GptImage1Request>),
  OpenaiGptImage1Mini(Vec<GptImage1MiniRequest>),
  OpenaiGptImage1_5(Vec<GptImage1_5Request>),
  OpenaiGptImage2(Vec<GptImage2Request>),
  GoogleNanoBanana(GoogleImageSlices<NanoBananaRequest>),
  GoogleNanoBananaPro(GoogleImageSlices<NanoBananaProRequest>),
  GoogleNanoBanana2(GoogleImageSlices<NanoBanana2Request>),
  GoogleNanoBanana2Lite(GoogleImageSlices<NanoBanana2LiteRequest>),
}

/// A build's result: the route plus every param adjustment made along the way.
#[derive(Debug)]
pub struct BuiltImageRoute {
  pub route: ImageRoute,
  pub adjustments: Vec<ParamAdjustment>,
}

impl ImageRoute {
  pub fn model(&self) -> RouterImageModel {
    match self {
      Self::OpenaiGptImage1(_) => RouterImageModel::OpenaiGptImageV1,
      Self::OpenaiGptImage1Mini(_) => RouterImageModel::OpenaiGptImageV1Mini,
      Self::OpenaiGptImage1_5(_) => RouterImageModel::OpenaiGptImageV1_5,
      Self::OpenaiGptImage2(_) => RouterImageModel::OpenaiGptImageV2,
      Self::GoogleNanoBanana(_) => RouterImageModel::GoogleNanoBanana,
      Self::GoogleNanoBananaPro(_) => RouterImageModel::GoogleNanoBananaPro,
      Self::GoogleNanoBanana2(_) => RouterImageModel::GoogleNanoBanana2,
      Self::GoogleNanoBanana2Lite(_) => RouterImageModel::GoogleNanoBanana2Lite,
    }
  }

  /// How this route's INPUT assets reach the provider: public CDN urls as-is
  /// (OpenAI) or upload-first to the provider's storage (Google — the
  /// dispatcher uploads `input_urls` and injects the refs per slice).
  pub fn input_strategy(&self) -> InputStrategy {
    input_strategy_for(self.provider())
  }

  /// The pending upload-first input urls (empty for PublicUrls providers).
  pub fn input_urls(&self) -> &[String] {
    match self {
      Self::GoogleNanoBanana(s) => &s.input_urls,
      Self::GoogleNanoBananaPro(s) => &s.input_urls,
      Self::GoogleNanoBanana2(s) => &s.input_urls,
      Self::GoogleNanoBanana2Lite(s) => &s.input_urls,
      _ => &[],
    }
  }

  /// Typed access to one Google slice — the outbound dispatcher's entry point
  /// (Nano Banana is SYNCHRONOUS at the provider: running a slice completes it).
  pub fn google_slice(&self, index: usize) -> Option<GoogleImageSliceRef<'_>> {
    match self {
      Self::GoogleNanoBanana(s) => s.requests.get(index).map(GoogleImageSliceRef::NanoBanana),
      Self::GoogleNanoBananaPro(s) => s.requests.get(index).map(GoogleImageSliceRef::NanoBananaPro),
      Self::GoogleNanoBanana2(s) => s.requests.get(index).map(GoogleImageSliceRef::NanoBanana2),
      Self::GoogleNanoBanana2Lite(s) => s.requests.get(index).map(GoogleImageSliceRef::NanoBanana2Lite),
      _ => None,
    }
  }

  pub fn provider(&self) -> catalog::Provider {
    self.model().spec().provider
  }

  /// How many provider calls this route fans out to (= the plan's `n`).
  pub fn fan_out(&self) -> usize {
    match self {
      Self::OpenaiGptImage1(reqs) => reqs.len(),
      Self::OpenaiGptImage1Mini(reqs) => reqs.len(),
      Self::OpenaiGptImage1_5(reqs) => reqs.len(),
      Self::OpenaiGptImage2(reqs) => reqs.len(),
      Self::GoogleNanoBanana(s) => s.requests.len(),
      Self::GoogleNanoBananaPro(s) => s.requests.len(),
      Self::GoogleNanoBanana2(s) => s.requests.len(),
      Self::GoogleNanoBanana2Lite(s) => s.requests.len(),
    }
  }

  /// Price the whole route (all slices), DELEGATING to the provider client's own cost
  /// calculators — the router never re-derives prices.
  pub fn price(&self) -> PriceQuote {
    use client_core::CostCalculator;
    // Delegate every slice to the provider client's estimator; the quote's
    // certainty is Exact only when EVERY slice priced exactly.
    let breakdowns: Vec<client_core::CostBreakdown> = match self {
      Self::OpenaiGptImage1(reqs) => reqs.iter().map(|r| r.estimate_cost()).collect(),
      Self::OpenaiGptImage1Mini(reqs) => reqs.iter().map(|r| r.estimate_cost()).collect(),
      Self::OpenaiGptImage1_5(reqs) => reqs.iter().map(|r| r.estimate_cost()).collect(),
      Self::OpenaiGptImage2(reqs) => reqs.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleNanoBanana(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleNanoBananaPro(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleNanoBanana2(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleNanoBanana2Lite(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
    };
    quote_from_breakdowns(&breakdowns, "image")
  }

  /// The absolute provider URL this route's requests post to — for outbound request logs.
  pub fn endpoint_url(&self) -> String {
    match self.provider() {
      catalog::Provider::Openai => openai_client::api::requests::images::RESPONSES_ENDPOINT_URL.to_string(),
      catalog::Provider::Google => format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        self.model().spec().provider_model_id
      ),
      catalog::Provider::Kinovi => "https://kinovi.ai/api/v1/jobs/createTask".to_string(),
    }
  }

  /// The EXACT provider-native JSON body slice `index` would send — built through the same
  /// path as the real send, so logs and dry runs record precisely what fires.
  pub fn request_json(&self, index: usize) -> Result<serde_json::Value, RouterError> {
    let out_of_range = || RouterError::Invalid(format!("slice index {index} out of range"));
    let json = match self {
      Self::OpenaiGptImage1(reqs) => {
        generate_gpt_image_1_wire_json(reqs.get(index).ok_or_else(out_of_range)?)
      }
      Self::OpenaiGptImage1Mini(reqs) => {
        generate_gpt_image_1_mini_wire_json(reqs.get(index).ok_or_else(out_of_range)?)
      }
      Self::OpenaiGptImage1_5(reqs) => {
        generate_gpt_image_1_5_wire_json(reqs.get(index).ok_or_else(out_of_range)?)
      }
      Self::OpenaiGptImage2(reqs) => {
        generate_gpt_image_2_wire_json(reqs.get(index).ok_or_else(out_of_range)?)
      }
      Self::GoogleNanoBanana(s) => {
        return generate_nano_banana_wire_json(s.requests.get(index).ok_or_else(out_of_range)?)
          .map_err(|e| RouterError::Provider(e.to_string()));
      }
      Self::GoogleNanoBananaPro(s) => {
        return generate_nano_banana_pro_wire_json(s.requests.get(index).ok_or_else(out_of_range)?)
          .map_err(|e| RouterError::Provider(e.to_string()));
      }
      Self::GoogleNanoBanana2(s) => {
        return generate_nano_banana_2_wire_json(s.requests.get(index).ok_or_else(out_of_range)?)
          .map_err(|e| RouterError::Provider(e.to_string()));
      }
      Self::GoogleNanoBanana2Lite(s) => {
        return generate_nano_banana_2_lite_wire_json(s.requests.get(index).ok_or_else(out_of_range)?)
          .map_err(|e| RouterError::Provider(e.to_string()));
      }
    };
    json.map_err(|e| RouterError::Provider(e.to_string()))
  }

  /// Start ONE slice in the OPENAI background queue; returns its handle. Google
  /// routes never come through here — Nano Banana is synchronous at the
  /// provider, so the google-gemini-outbound-job runs slices to COMPLETION via
  /// [`Self::google_slice`] instead of starting them.
  pub async fn start_slice(&self, api_key: &OpenAiApiKey, index: usize) -> Result<ProviderHandle, RouterError> {
    let out_of_range = || RouterError::Invalid(format!("slice index {index} out of range"));
    let started = match self {
      Self::OpenaiGptImage1(reqs) => {
        let request = reqs.get(index).ok_or_else(out_of_range)?.clone();
        generate_gpt_image_1(GenerateGptImage1Args { api_key, request }).await
      }
      Self::OpenaiGptImage1Mini(reqs) => {
        let request = reqs.get(index).ok_or_else(out_of_range)?.clone();
        generate_gpt_image_1_mini(GenerateGptImage1MiniArgs { api_key, request }).await
      }
      Self::OpenaiGptImage1_5(reqs) => {
        let request = reqs.get(index).ok_or_else(out_of_range)?.clone();
        generate_gpt_image_1_5(GenerateGptImage1_5Args { api_key, request }).await
      }
      Self::OpenaiGptImage2(reqs) => {
        let request = reqs.get(index).ok_or_else(out_of_range)?.clone();
        generate_gpt_image_2(GenerateGptImage2Args { api_key, request }).await
      }
      Self::GoogleNanoBanana(_)
      | Self::GoogleNanoBananaPro(_)
      | Self::GoogleNanoBanana2(_)
      | Self::GoogleNanoBanana2Lite(_) => {
        return Err(RouterError::Invalid(
          "google image routes are synchronous — dispatch them via google_slice() in the outbound job".into(),
        ));
      }
    }
    .map_err(|e| RouterError::Provider(e.to_string()))?;
    Ok(ProviderHandle { provider_job_id: started.response_id.0 })
  }

  /// Run ONE Google slice to COMPLETION (Nano Banana is synchronous at the
  /// provider). `inputs` are the Files-API refs the dispatcher uploaded for
  /// this job — injected into the request in place of the pending CDN urls.
  pub async fn run_google_slice(
    &self,
    api_key: &GoogleGeminiApiKey,
    index: usize,
    inputs: &[ImageInput],
  ) -> Result<Vec<RoutedImage>, RouterError> {
    let out_of_range = || RouterError::Invalid(format!("slice index {index} out of range"));
    let success = match self {
      Self::GoogleNanoBanana(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.images = inputs.to_vec();
        generate_nano_banana(GenerateNanoBananaArgs { api_key, request }).await
      }
      Self::GoogleNanoBananaPro(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.images = inputs.to_vec();
        generate_nano_banana_pro(GenerateNanoBananaProArgs { api_key, request }).await
      }
      Self::GoogleNanoBanana2(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.images = inputs.to_vec();
        generate_nano_banana_2(GenerateNanoBanana2Args { api_key, request }).await
      }
      Self::GoogleNanoBanana2Lite(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.images = inputs.to_vec();
        generate_nano_banana_2_lite(GenerateNanoBanana2LiteArgs { api_key, request }).await
      }
      _ => {
        return Err(RouterError::Invalid(
          "not a google image route — openai routes dispatch via start_slice()".into(),
        ));
      }
    }
    .map_err(|e| RouterError::Provider(e.to_string()))?;
    Ok(success.images.iter().map(|i| routed_image_from(&i.mime, i.bytes.clone())).collect())
  }
}

/// Normalize a provider-reported mime onto the bucket-path (mime, extension)
/// vocabulary; unknown types are stored as PNG (Gemini emits png/jpeg/webp).
fn routed_image_from(mime: &str, bytes: Vec<u8>) -> RoutedImage {
  let (mime, extension) = match mime {
    "image/jpeg" => ("image/jpeg", ".jpg"),
    "image/webp" => ("image/webp", ".webp"),
    _ => ("image/png", ".png"),
  };
  RoutedImage { bytes, mime, extension }
}

/// Assemble a [`PriceQuote`] from per-slice breakdowns: totals summed, the
/// quote Exact only when every slice was.
pub(crate) fn quote_from_breakdowns(breakdowns: &[client_core::CostBreakdown], unit_label: &str) -> PriceQuote {
  use client_core::{CostAmount, CostCertainty};
  let total: i64 = breakdowns.iter().map(|b| b.amount.micros().0).sum();
  let all_exact = breakdowns.iter().all(|b| b.amount.certainty() == CostCertainty::Exact);
  let max: i64 = breakdowns
    .iter()
    .map(|b| match b.amount {
      CostAmount::Exact(m) | CostAmount::Estimated(m) => m.0,
    })
    .sum();
  PriceQuote {
    certainty: if all_exact { PriceCertainty::Exact } else { PriceCertainty::Estimated },
    amount_micros: total,
    max_micros: if all_exact { Some(max) } else { None },
    line_items: breakdowns
      .iter()
      .enumerate()
      .map(|(i, b)| PriceLineItem {
        label: format!("{unit_label} {} of {}", i + 1, breakdowns.len()),
        amount_micros: b.amount.micros().0,
      })
      .collect(),
  }
}
