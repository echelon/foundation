use client_core::{CostCalculator, Micros};

use crate::api::requests::images::image_generation::image_generation::ImageGenerationRequest;
use crate::api::requests::images::per_image_micros;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;

/// `image_generation` is pure text → image: only an OUTPUT cost, per image, by
/// (model, quality), scaled by `n`. Defaults: model = gpt-image-1, quality = medium, n = 1.
impl CostCalculator for ImageGenerationRequest {
  fn cost_micros(&self) -> Micros {
    let model = self.model.clone().unwrap_or(ImageModel::GptImage1);
    let quality = self.quality.unwrap_or(ImageQuality::Medium);
    let n = self.number_images.unwrap_or(1) as i64;
    Micros(per_image_micros(&model, quality) * n)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::image_types::image_size::ImageSize;
  use crate::api::types::image_types::output_format::OutputFormat;

  fn req(model: Option<ImageModel>, quality: Option<ImageQuality>, n: Option<u32>) -> ImageGenerationRequest {
    ImageGenerationRequest {
      prompt: "t".into(),
      model,
      number_images: n,
      size: None,
      quality,
      output_format: None,
      transparent_background: None,
      user: None,
    }
  }

  mod per_image_rates {
    use super::*;

    #[test]
    fn gpt_image_1_medium_is_4c() {
      assert_eq!(req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1)).cost_micros(), Micros(40_000));
    }
    #[test]
    fn gpt_image_1_high_is_8c() {
      assert_eq!(req(Some(ImageModel::GptImage1), Some(ImageQuality::High), Some(1)).cost_micros(), Micros(80_000));
    }
    #[test]
    fn gpt_image_1_5_cheaper_than_1() {
      let v1 = req(Some(ImageModel::GptImage1), Some(ImageQuality::High), Some(1)).cost_micros();
      let v15 = req(Some(ImageModel::GptImage1_5), Some(ImageQuality::High), Some(1)).cost_micros();
      assert!(v15.0 < v1.0);
    }
    #[test]
    fn gpt_image_2_priciest_high() {
      assert_eq!(req(Some(ImageModel::GptImage2), Some(ImageQuality::High), Some(1)).cost_micros(), Micros(120_000));
    }
  }

  mod defaults {
    use super::*;
    #[test]
    fn default_model_is_gpt_image_1_medium() {
      assert_eq!(req(None, None, None).cost_micros(), Micros(40_000));
    }
    #[test]
    fn auto_quality_bills_as_medium() {
      let a = req(Some(ImageModel::GptImage1), Some(ImageQuality::Auto), Some(1)).cost_micros();
      let m = req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1)).cost_micros();
      assert_eq!(a, m);
    }
  }

  mod scaling {
    use super::*;
    #[test]
    fn n_scales_linearly() {
      for n in 1u32..=10 {
        assert_eq!(
          req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(n)).cost_micros(),
          Micros(40_000 * n as i64),
        );
      }
    }
  }

  mod independence {
    use super::*;
    #[test]
    fn cost_ignores_size_format_user() {
      let base = req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1)).cost_micros();
      let mut r = req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1));
      r.size = Some(ImageSize::Landscape1536x1024);
      r.output_format = Some(OutputFormat::Webp);
      r.user = Some("u".into());
      assert_eq!(r.cost_micros(), base);
    }
  }

  mod monotonicity {
    use super::*;
    #[test]
    fn quality_is_monotonic() {
      for m in [ImageModel::GptImage1, ImageModel::GptImage1_5, ImageModel::GptImage2] {
        let lo = req(Some(m.clone()), Some(ImageQuality::Low), Some(1)).cost_micros();
        let mid = req(Some(m.clone()), Some(ImageQuality::Medium), Some(1)).cost_micros();
        let hi = req(Some(m.clone()), Some(ImageQuality::High), Some(1)).cost_micros();
        assert!(lo.0 <= mid.0 && mid.0 <= hi.0, "not monotonic for {}", m.as_str());
      }
    }
  }
}
