use client_core::{CostCalculator, Micros};

use crate::api::requests::images::generate_image::generate_image::GenerateImageRequest;
use crate::api::types::image_types::gemini_image_model::GeminiImageModel;

// Google's published per-image rates for the Nano Banana family (micro-USD):
//
//   gemini-2.5-flash-image   (Nano Banana)      $0.02 / img  = 20_000 micros
//   gemini-3.1-flash-image   (Nano Banana 2)    $0.04 / img  = 40_000 micros
//   gemini-3-pro-image       (Nano Banana Pro)  $0.05 / img  = 50_000 micros
//
// A `Custom` model conservatively assumes the most expensive Pro tier.
//
// `generate_image` bills per output image; the number of images is Gemini's
// `candidateCount` (default 1). Cost is independent of prompt, aspect ratio,
// image size, and any reference-image inputs.

const NANO_BANANA_MICROS_PER_IMAGE: i64 = 20_000;
const NANO_BANANA_2_MICROS_PER_IMAGE: i64 = 40_000;
const NANO_BANANA_PRO_MICROS_PER_IMAGE: i64 = 50_000;
const CUSTOM_MICROS_PER_IMAGE: i64 = 50_000;

impl CostCalculator for GenerateImageRequest {
  fn cost_micros(&self) -> Micros {
    let per_image = micros_per_image(self.model.as_ref().unwrap_or(&GeminiImageModel::NanoBanana));

    // Server default for `candidateCount` is 1.
    let count = self.candidate_count.unwrap_or(1) as i64;

    Micros(per_image * count)
  }
}

/// Per-image output rate in micro-USD for a given model.
fn micros_per_image(model: &GeminiImageModel) -> i64 {
  match model {
    GeminiImageModel::NanoBanana    => NANO_BANANA_MICROS_PER_IMAGE,
    GeminiImageModel::NanoBanana2   => NANO_BANANA_2_MICROS_PER_IMAGE,
    GeminiImageModel::NanoBananaPro => NANO_BANANA_PRO_MICROS_PER_IMAGE,
    GeminiImageModel::Custom(_)     => CUSTOM_MICROS_PER_IMAGE,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::images::generate_image::generate_image::ImageInput;
  use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
  use crate::api::types::image_types::image_size::ImageSize;

  fn make_request(model: Option<GeminiImageModel>, candidate_count: Option<u32>) -> GenerateImageRequest {
    GenerateImageRequest {
      prompt: "test".to_string(),
      model,
      aspect_ratio: None,
      image_size: None,
      candidate_count,
      images: vec![],
    }
  }

  // ── Per-model rate (single image) ──

  mod per_model_rates {
    use super::*;

    #[test]
    fn nano_banana_is_20_000_micros() {
      let req = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      assert_eq!(req.cost_micros(), Micros(20_000));
    }

    #[test]
    fn nano_banana_2_is_40_000_micros() {
      let req = make_request(Some(GeminiImageModel::NanoBanana2), Some(1));
      assert_eq!(req.cost_micros(), Micros(40_000));
    }

    #[test]
    fn nano_banana_pro_is_50_000_micros() {
      let req = make_request(Some(GeminiImageModel::NanoBananaPro), Some(1));
      assert_eq!(req.cost_micros(), Micros(50_000));
    }

    #[test]
    fn custom_model_uses_pro_rate() {
      let custom = GeminiImageModel::Custom("gemini-future-image".to_string());
      let r_custom = make_request(Some(custom), Some(1));
      let r_pro = make_request(Some(GeminiImageModel::NanoBananaPro), Some(1));
      assert_eq!(r_custom.cost_micros(), r_pro.cost_micros());
      assert_eq!(r_custom.cost_micros(), Micros(50_000));
    }
  }

  // ── Defaults ──

  mod defaults {
    use super::*;

    #[test]
    fn default_model_is_nano_banana() {
      let r1 = make_request(None, Some(1));
      let r2 = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      assert_eq!(r1.cost_micros(), r2.cost_micros());
      assert_eq!(r1.cost_micros(), Micros(20_000));
    }

    #[test]
    fn default_candidate_count_is_one() {
      let r1 = make_request(Some(GeminiImageModel::NanoBanana), None);
      let r2 = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      assert_eq!(r1.cost_micros(), r2.cost_micros());
    }

    #[test]
    fn fully_default_request_costs_20_000_micros() {
      // All defaults = gemini-2.5-flash-image, candidateCount=1 → 20_000 micros = $0.02.
      let req = make_request(None, None);
      assert_eq!(req.cost_micros(), Micros(20_000));
      assert_eq!(req.cost_micros().as_usd(), 0.02);
    }
  }

  // ── Batch scaling ──

  mod batch_scaling {
    use super::*;

    #[test]
    fn count_scales_linearly_nano_banana() {
      for n in 1u32..=10 {
        let req = make_request(Some(GeminiImageModel::NanoBanana), Some(n));
        assert_eq!(req.cost_micros(), Micros(20_000 * n as i64), "n={n}");
      }
    }

    #[test]
    fn count_scales_linearly_pro() {
      for n in 1u32..=10 {
        let req = make_request(Some(GeminiImageModel::NanoBananaPro), Some(n));
        assert_eq!(req.cost_micros(), Micros(50_000 * n as i64), "n={n}");
      }
    }

    #[test]
    fn batch_of_four_pro() {
      let req = make_request(Some(GeminiImageModel::NanoBananaPro), Some(4));
      // 50_000 × 4 = 200_000 micros = $0.20.
      assert_eq!(req.cost_micros(), Micros(200_000));
      assert_eq!(req.cost_micros().as_usd(), 0.20);
    }
  }

  // ── Cost doesn't depend on irrelevant fields ──

  mod independence {
    use super::*;

    #[test]
    fn cost_is_independent_of_aspect_ratio() {
      let mut base = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      let base_cost = base.cost_micros();
      for ar in [
        ImageAspectRatio::Square,
        ImageAspectRatio::Landscape16x9,
        ImageAspectRatio::Portrait9x16,
        ImageAspectRatio::Landscape21x9,
      ] {
        base.aspect_ratio = Some(ar);
        assert_eq!(base.cost_micros(), base_cost, "{ar:?}");
      }
    }

    #[test]
    fn cost_is_independent_of_image_size() {
      let mut base = make_request(Some(GeminiImageModel::NanoBananaPro), Some(1));
      let base_cost = base.cost_micros();
      for sz in [ImageSize::OneK, ImageSize::TwoK, ImageSize::FourK] {
        base.image_size = Some(sz);
        assert_eq!(base.cost_micros(), base_cost, "{sz:?}");
      }
    }

    #[test]
    fn cost_is_independent_of_reference_images() {
      let mut base = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      let base_cost = base.cost_micros();
      base.images = vec![ImageInput { mime: "image/png".to_string(), data: vec![1, 2, 3, 4] }];
      assert_eq!(base.cost_micros(), base_cost);
    }

    #[test]
    fn cost_is_independent_of_prompt() {
      let mut base = make_request(Some(GeminiImageModel::NanoBanana), Some(1));
      let base_cost = base.cost_micros();
      base.prompt = "a completely different and much longer prompt".to_string();
      assert_eq!(base.cost_micros(), base_cost);
    }
  }

  // ── Monotonicity sanity checks ──

  mod monotonicity {
    use super::*;

    #[test]
    fn flash_tiers_are_no_more_expensive_than_pro() {
      let pro = make_request(Some(GeminiImageModel::NanoBananaPro), Some(1)).cost_micros();
      let flash = make_request(Some(GeminiImageModel::NanoBanana), Some(1)).cost_micros();
      let flash2 = make_request(Some(GeminiImageModel::NanoBanana2), Some(1)).cost_micros();
      assert!(flash <= pro);
      assert!(flash2 <= pro);
      assert!(flash <= flash2);
    }
  }
}
