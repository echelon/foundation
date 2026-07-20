use crate::text_enum::text_enum;

text_enum! {
  /// What a user's avatar displays (accounts.md §2). `NotYetSet` is the signup default: the
  /// system auto-picks — gravatar when one is mirrored, else the letter monogram. Any explicit
  /// user choice replaces it and is honored from then on.
  pub enum AvatarSource {
    NotYetSet => "not_yet_set",
    Custom => "custom",
    Gravatar => "gravatar",
    Letter => "letter",
    Generated => "generated",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["not_yet_set", "custom", "gravatar", "letter", "generated"];
    assert_eq!(AvatarSource::ALL.len(), expect.len());
    for (v, s) in AvatarSource::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<AvatarSource>().unwrap(), *v);
      assert_eq!(serde_json::to_string(v).unwrap(), format!("\"{s}\""));
    }
  }

  #[test]
  fn unknown_value_rejected() {
    assert!("banana".parse::<AvatarSource>().is_err());
  }
}
