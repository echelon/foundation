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

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::plan::RouterImageModel;
use crate::pricing::{PriceCertainty, PriceLineItem, PriceQuote};

/// A provider-side handle for ONE slice (for OpenAI: the background response id).
#[derive(Clone, Debug)]
pub struct ProviderHandle {
  pub provider_job_id: String,
}

/// The built, ready-to-dispatch route. One variant per catalog model (hard-coded).
#[derive(Debug)]
pub enum ImageRoute {
  OpenaiGptImage1(Vec<GptImage1Request>),
  OpenaiGptImage1Mini(Vec<GptImage1MiniRequest>),
  OpenaiGptImage1_5(Vec<GptImage1_5Request>),
  OpenaiGptImage2(Vec<GptImage2Request>),
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
    }
  }

  /// Price the whole route (all slices), DELEGATING to the provider client's own cost
  /// calculators — the router never re-derives prices.
  pub fn price(&self) -> PriceQuote {
    use client_core::BetterCostEstimator;
    let per_slice: Vec<i64> = match self {
      Self::OpenaiGptImage1(reqs) => reqs.iter().map(|r| r.estimate_cost().amount.micros().0).collect(),
      Self::OpenaiGptImage1Mini(reqs) => reqs.iter().map(|r| r.estimate_cost().amount.micros().0).collect(),
      Self::OpenaiGptImage1_5(reqs) => reqs.iter().map(|r| r.estimate_cost().amount.micros().0).collect(),
      Self::OpenaiGptImage2(reqs) => reqs.iter().map(|r| r.estimate_cost().amount.micros().0).collect(),
    };
    let total: i64 = per_slice.iter().sum();
    PriceQuote {
      // GPT Image is token-metered underneath: 1024² rates are a good-faith estimate,
      // larger sizes / edit inputs can bill more. Post-call usage is authoritative.
      certainty: PriceCertainty::Estimated,
      amount_micros: total,
      max_micros: None,
      line_items: per_slice
        .iter()
        .enumerate()
        .map(|(i, m)| PriceLineItem { label: format!("image {} of {}", i + 1, per_slice.len()), amount_micros: *m })
        .collect(),
    }
  }

  /// The absolute provider URL this route's requests post to — for outbound request logs.
  pub fn endpoint_url(&self) -> &'static str {
    openai_client::api::requests::images::RESPONSES_ENDPOINT_URL
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
    };
    json.map_err(|e| RouterError::Provider(e.to_string()))
  }

  /// Start ONE slice in the provider's background queue; returns its handle. The caller
  /// (the dispatcher) owns iteration, persistence, and retries per slice.
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
    }
    .map_err(|e| RouterError::Provider(e.to_string()))?;
    Ok(ProviderHandle { provider_job_id: started.response_id.0 })
  }
}
