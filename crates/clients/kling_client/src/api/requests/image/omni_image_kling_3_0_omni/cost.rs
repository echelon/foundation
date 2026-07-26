use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::image::omni_image_kling_3_0_omni::omni_image_kling_3_0_omni::{
  OmniImageKling30OmniRequest, OmniImageResult,
};
use crate::api::types::image_types::image_resolution::ImageResolution;
use crate::api::types::image_types::series_amount::SeriesAmount;

/// Official pricing (pricing/base/image): Kling Image 3.0 Omni — $0.028 per
/// image at 1K/2K, $0.056 at 4K. Billed per generated image.
const PER_IMAGE_1K_2K_MICROS: i64 = 28_000;
const PER_IMAGE_4K_MICROS: i64 = 56_000;

/// `series_amount: auto` generates a model-chosen count — estimates assume
/// the documented default of 4.
const ASSUMED_AUTO_SERIES_IMAGES: f64 = 4.0;

impl CostCalculator for OmniImageKling30OmniRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let per_image = match self.resolution.unwrap_or(ImageResolution::R1k) {
      ImageResolution::R1k | ImageResolution::R2k => PER_IMAGE_1K_2K_MICROS,
      ImageResolution::R4k => PER_IMAGE_4K_MICROS,
    };
    let (quantity, label, force_estimated) = match self.result {
      OmniImageResult::Single { n } => {
        let n = n.unwrap_or(1);
        (CostQuantity::Exact(n as f64), format!("Image output ({n} image(s))"), false)
      }
      OmniImageResult::Series { amount } => match amount {
        Some(SeriesAmount::Count(count)) => {
          (CostQuantity::Exact(count as f64), format!("Series output ({count} images)"), false)
        }
        Some(SeriesAmount::Auto) | None => (
          CostQuantity::Estimated(ASSUMED_AUTO_SERIES_IMAGES),
          format!("Series output (~{ASSUMED_AUTO_SERIES_IMAGES} images, model-chosen)"),
          true,
        ),
      },
    };
    let mut component = CostComponent::new(label, CostUnit::PerImage, per_image, quantity);
    if force_estimated {
      component = component.with_note("auto series count is model-chosen; task billing is authoritative");
    }
    CostBreakdown::from_components(
      CostFormula {
        display: "images × rate(resolution)",
        rates: vec![
          RateRow {
            dimension: "resolution",
            value: "1k/2k".to_string(),
            unit: CostUnit::PerImage,
            price_micros: PER_IMAGE_1K_2K_MICROS,
          },
          RateRow {
            dimension: "resolution",
            value: "4k".to_string(),
            unit: CostUnit::PerImage,
            price_micros: PER_IMAGE_4K_MICROS,
          },
        ],
      },
      vec![component],
      force_estimated,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;

  fn request() -> OmniImageKling30OmniRequest {
    OmniImageKling30OmniRequest::new("a fox")
  }

  #[test]
  fn published_rates() {
    assert_eq!(request().estimate_cost().amount.micros(), MicroUsd(28_000));

    let mut four_k = request();
    four_k.resolution = Some(ImageResolution::R4k);
    assert_eq!(four_k.estimate_cost().amount.micros(), MicroUsd(56_000));
  }

  #[test]
  fn n_and_fixed_series_are_exact_multiples() {
    let mut batch = request();
    batch.result = OmniImageResult::Single { n: Some(3) };
    assert_eq!(batch.estimate_cost().amount.micros(), MicroUsd(84_000));
    assert_eq!(batch.estimate_cost().amount.certainty(), CostCertainty::Exact);

    let mut series = request();
    series.result = OmniImageResult::Series { amount: Some(SeriesAmount::Count(9)) };
    assert_eq!(series.estimate_cost().amount.micros(), MicroUsd(9 * 28_000));
  }

  #[test]
  fn auto_series_is_estimated_at_4_images() {
    let mut series = request();
    series.result = OmniImageResult::Series { amount: Some(SeriesAmount::Auto) };
    let breakdown = series.estimate_cost();
    assert_eq!(breakdown.amount.micros(), MicroUsd(4 * 28_000));
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
  }

  #[test]
  fn matches_plain_image_3_0_rate_at_1k(){
    use crate::api::requests::image::generate_image_kling_3_0::generate_image_kling_3_0::GenerateImageKling30Request;
    let plain = GenerateImageKling30Request::new("a fox").estimate_cost().amount.micros();
    assert_eq!(request().estimate_cost().amount.micros(), plain);
  }
}
