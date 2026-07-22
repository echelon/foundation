use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::NanoBanana2Request;
use crate::api::types::image_types::nano_banana_2_image_size::NanoBanana2ImageSize;

// Nano Banana 2 (gemini-3.1-flash-image): $0.045 (512px) / $0.067 (1K, default) / $0.101 (2K) / $0.151 (4K) per image.

impl CostCalculator for NanoBanana2Request {
  fn cost_micros(&self) -> Micros {
    match self.image_size.unwrap_or(NanoBanana2ImageSize::K1) {
    NanoBanana2ImageSize::P512 => Micros(45_000),
    NanoBanana2ImageSize::K1 => Micros(67_000),
    NanoBanana2ImageSize::K2 => Micros(101_000),
    NanoBanana2ImageSize::K4 => Micros(151_000),
  }
  }

  /// Thinking tokens are billed by default on top of the per-image tier — provider usage is authoritative.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Estimated
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(size: Option<NanoBanana2ImageSize>) -> NanoBanana2Request {
    NanoBanana2Request { prompt: "p".into(), images: vec![], aspect_ratio: None, image_size: size }
  }

  #[test]
  fn tiers_follow_image_size() {
    assert_eq!(req(None).cost_micros(), Micros(67_000)); // default 1K
    assert_eq!(req(Some(NanoBanana2ImageSize::P512)).cost_micros(), Micros(45_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K1)).cost_micros(), Micros(67_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K2)).cost_micros(), Micros(101_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K4)).cost_micros(), Micros(151_000));
  }

  #[test]
  fn estimated_because_thinking_tokens_bill_on_top() {
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}
