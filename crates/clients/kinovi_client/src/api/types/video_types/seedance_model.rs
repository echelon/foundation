/// ByteDance Seedance video models exposed through Kinovi's `createTask` endpoint.
///
/// Kinovi may add or rename models faster than this crate ships releases, so the
/// [`SeedanceModel::Custom`] escape hatch lets callers pass an arbitrary model id
/// without waiting for a code change.
///
/// The wire string is the exact value Kinovi expects in the top-level `"model"`
/// field. Per-second pricing lives next to the endpoint in
/// [`crate::api::requests::video::create_task::cost`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedanceModel {
  /// `seedance-20` — Seedance 2.0, the flagship tier ($0.12 / video-second).
  Seedance20,

  /// `seedance2-fast` — the faster, cheaper tier ($0.06 / video-second).
  Seedance2Fast,

  /// `seedance2.0-mini` — the cheapest tier ($0.03 / video-second).
  Seedance20Mini,

  /// Escape hatch for model identifiers not yet enumerated here. Priced at the
  /// flagship rate by the cost calculator (conservative).
  Custom(String),
}

impl SeedanceModel {
  /// Wire representation — the exact string Kinovi expects in the `"model"` field.
  pub fn as_str(&self) -> &str {
    match self {
      Self::Seedance20     => "seedance-20",
      Self::Seedance2Fast  => "seedance2-fast",
      Self::Seedance20Mini => "seedance2.0-mini",
      Self::Custom(s)      => s.as_str(),
    }
  }
}

// Serialize as the wire string rather than the default external-tag enum form,
// so `CreateTaskRequest` round-trips through any log/audit pipeline as readable JSON.
impl serde::Serialize for SeedanceModel {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn known_models_map_to_docs_strings() {
    assert_eq!(SeedanceModel::Seedance20.as_str(), "seedance-20");
    assert_eq!(SeedanceModel::Seedance2Fast.as_str(), "seedance2-fast");
    assert_eq!(SeedanceModel::Seedance20Mini.as_str(), "seedance2.0-mini");
  }

  #[test]
  fn custom_model_passes_through() {
    let m = SeedanceModel::Custom("seedance-30-preview".to_string());
    assert_eq!(m.as_str(), "seedance-30-preview");
  }

  #[test]
  fn known_models_are_distinct_strings() {
    let all = [
      SeedanceModel::Seedance20.as_str(),
      SeedanceModel::Seedance2Fast.as_str(),
      SeedanceModel::Seedance20Mini.as_str(),
    ];
    let mut v: Vec<&str> = all.to_vec();
    v.sort();
    v.dedup();
    assert_eq!(v.len(), 3);
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&SeedanceModel::Seedance20).unwrap(), "\"seedance-20\"");
    assert_eq!(serde_json::to_string(&SeedanceModel::Seedance2Fast).unwrap(), "\"seedance2-fast\"");
    assert_eq!(serde_json::to_string(&SeedanceModel::Seedance20Mini).unwrap(), "\"seedance2.0-mini\"");
    assert_eq!(
      serde_json::to_string(&SeedanceModel::Custom("x".to_string())).unwrap(),
      "\"x\""
    );
  }
}
