use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana_pro::generate_nano_banana_pro::NanoBananaProRequest;
use crate::api::types::image_types::nano_banana_pro_image_size::NanoBananaProImageSize;

// Nano Banana Pro (gemini-3-pro-image): $0.134 (1K/2K, default 1K) / $0.24 (4K) per image.

impl CostCalculator for NanoBananaProRequest {
  fn cost_micros(&self) -> Micros {
    match self.image_size.unwrap_or(NanoBananaProImageSize::K1) {
    NanoBananaProImageSize::K1 | NanoBananaProImageSize::K2 => Micros(134_000),
    NanoBananaProImageSize::K4 => Micros(240_000),
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

  fn req(size: Option<NanoBananaProImageSize>) -> NanoBananaProRequest {
    NanoBananaProRequest { prompt: "p".into(), images: vec![], aspect_ratio: None, image_size: size }
  }

  #[test]
  fn tiers_follow_image_size() {
    assert_eq!(req(None).cost_micros(), Micros(134_000)); // default 1K
    assert_eq!(req(Some(NanoBananaProImageSize::K1)).cost_micros(), Micros(134_000));
    assert_eq!(req(Some(NanoBananaProImageSize::K2)).cost_micros(), Micros(134_000));
    assert_eq!(req(Some(NanoBananaProImageSize::K4)).cost_micros(), Micros(240_000));
  }

  #[test]
  fn estimated_because_thinking_tokens_bill_on_top() {
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}
