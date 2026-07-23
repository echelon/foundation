use client_core::{
  BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow,
};

use crate::api::requests::images::generate_gpt_image_2::generate_gpt_image_2::GptImage2Request;
use crate::api::requests::images::per_image_micros;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;

// gpt-image-2: per-image rates at 1024×1024 by quality (`Auto` bills as medium).
// GPT Image is token-metered underneath: larger sizes and edit INPUT images bill
// additional tokens not visible in this request, and the tool's LLM driver adds
// a sliver — so the total is an ESTIMATE; the response `usage` is authoritative.

impl BetterCostEstimator for GptImage2Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let quality = self.quality.unwrap_or(ImageQuality::Medium);
    let per_image = per_image_micros(&ImageModel::GptImage2, quality);
    CostBreakdown::from_components(
      CostFormula {
        display: "1 image × rate(quality) at 1024×1024 + hidden input/driver tokens",
        rates: [ImageQuality::Low, ImageQuality::Medium, ImageQuality::High]
          .iter()
          .map(|q| RateRow {
            dimension: "quality",
            value: format!("{q:?}").to_lowercase(),
            unit: CostUnit::PerImage,
            price_micros: per_image_micros(&ImageModel::GptImage2, *q),
          })
          .collect(),
      },
      vec![
        CostComponent::new(
          format!("Output image ({quality:?} @ 1024×1024)"),
          CostUnit::PerImage,
          per_image,
          CostQuantity::Exact(1.0),
        )
        .with_note("token-metered underneath; larger sizes/edit inputs bill extra — usage is authoritative"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;

  fn req(quality: Option<ImageQuality>) -> GptImage2Request {
    let mut r = GptImage2Request::new("t");
    r.quality = quality;
    r
  }

  #[test]
  fn published_rates_at_1024() {
    assert_eq!(req(Some(ImageQuality::Low)).estimate_cost().amount.micros(), MicroUsd(6_000));
    assert_eq!(req(Some(ImageQuality::Medium)).estimate_cost().amount.micros(), MicroUsd(53_000));
    assert_eq!(req(Some(ImageQuality::High)).estimate_cost().amount.micros(), MicroUsd(211_000));
  }

  #[test]
  fn default_and_auto_bill_as_medium_and_totals_are_estimates() {
    assert_eq!(req(None).estimate_cost().amount.micros(), MicroUsd(53_000));
    assert_eq!(req(Some(ImageQuality::Auto)).estimate_cost().amount.micros(), MicroUsd(53_000));
    // Token-metered beneath the flat rate — never claim exactness.
    assert_eq!(req(None).estimate_cost().amount.certainty(), CostCertainty::Estimated);
    assert_eq!(req(None).estimate_cost().formula.rates.len(), 3);
  }
}
