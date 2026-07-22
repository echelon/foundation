use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana::generate_nano_banana::NanoBananaRequest;

// Nano Banana (gemini-2.5-flash-image): $0.039 per image, flat (1024px, ~1,290 output tokens).

impl CostCalculator for NanoBananaRequest {
  fn cost_micros(&self) -> Micros {
    Micros(39_000)
  }

  /// Flat per-image rate — the params fully determine the charge.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Exact
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req() -> NanoBananaRequest {
    NanoBananaRequest { prompt: "p".into(), images: vec![], aspect_ratio: None }
  }

  #[test]
  fn flat_per_image() {
    assert_eq!(req().cost_micros(), Micros(39_000)); // $0.039
    assert_eq!(req().cost_certainty(), CostCertainty::Exact);
  }
}
