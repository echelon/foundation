/// Output image encoding for GPT Image (`output_format`). Determines the returned MIME.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
  Png,
  Jpeg,
  Webp,
}

impl OutputFormat {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Png => "png",
      Self::Jpeg => "jpeg",
      Self::Webp => "webp",
    }
  }

  /// The MIME type of the decoded bytes.
  pub fn mime(&self) -> &'static str {
    match self {
      Self::Png => "image/png",
      Self::Jpeg => "image/jpeg",
      Self::Webp => "image/webp",
    }
  }

  /// Parse the `output_format` field echoed back in the response.
  pub fn from_wire(s: &str) -> OutputFormat {
    match s {
      "jpeg" | "jpg" => Self::Jpeg,
      "webp" => Self::Webp,
      _ => Self::Png,
    }
  }
}

impl serde::Serialize for OutputFormat {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mime_mapping() {
    assert_eq!(OutputFormat::Png.mime(), "image/png");
    assert_eq!(OutputFormat::Jpeg.mime(), "image/jpeg");
    assert_eq!(OutputFormat::Webp.mime(), "image/webp");
  }

  #[test]
  fn from_wire_defaults_png() {
    assert_eq!(OutputFormat::from_wire("weird"), OutputFormat::Png);
    assert_eq!(OutputFormat::from_wire("jpg"), OutputFormat::Jpeg);
  }
}
