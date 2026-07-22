use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::music::generate_lyria_3_clip::generate_lyria_3_clip::Lyria3ClipRequest;

// Lyria 3 Clip (lyria-3-clip-preview): $0.04 per song (per request).

impl CostCalculator for Lyria3ClipRequest {
  fn cost_micros(&self) -> Micros {
    Micros(40_000)
  }

  /// Flat per-request pricing — the params fully determine the charge.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Exact
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn flat_per_song() {
    let request = Lyria3ClipRequest { prompt: "p".into(), images: vec![] };
    assert_eq!(request.cost_micros(), Micros(40_000));
    assert_eq!(request.cost_certainty(), CostCertainty::Exact);
  }
}
