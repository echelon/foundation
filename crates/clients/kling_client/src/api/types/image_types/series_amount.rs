use serde::Serialize;

/// The omni-image `series_amount`: a fixed count (2–9) or `auto` (the model
/// decides; concurrency and billing follow the actually generated count).
/// The wire type is number-or-string, hence the hand-written Serialize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesAmount {
  Count(u8),
  Auto,
}

impl Serialize for SeriesAmount {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    match self {
      Self::Count(n) => serializer.serialize_u8(*n),
      Self::Auto => serializer.serialize_str("auto"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_number_or_auto_string() {
    assert_eq!(serde_json::to_value(SeriesAmount::Count(4)).unwrap(), serde_json::json!(4));
    assert_eq!(serde_json::to_value(SeriesAmount::Auto).unwrap(), serde_json::json!("auto"));
  }
}
