use crate::text_enum::text_enum;

text_enum! {
  /// The initials-monogram gradient, user-picked in settings (`user_avatars.monogram_gradient`;
  /// null = the default `brand`). Slugs only — the actual color stops live in the frontend.
  pub enum MonogramGradient {
    Brand => "brand",
    Ocean => "ocean",
    Sunset => "sunset",
    Forest => "forest",
    Berry => "berry",
    Gold => "gold",
    Slate => "slate",
    Rose => "rose",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["brand", "ocean", "sunset", "forest", "berry", "gold", "slate", "rose"];
    assert_eq!(MonogramGradient::ALL.len(), expect.len());
    for (v, s) in MonogramGradient::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<MonogramGradient>().unwrap(), *v);
    }
  }
}
