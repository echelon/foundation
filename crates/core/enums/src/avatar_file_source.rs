use crate::text_enum::text_enum;

text_enum! {
  /// Where the bytes of a `user_avatar_files` row came from (accounts.md §2).
  pub enum AvatarFileSource {
    CustomUpload => "custom_upload",
    GravatarMirror => "gravatar_mirror",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    assert_eq!(AvatarFileSource::CustomUpload.as_str(), "custom_upload");
    assert_eq!(AvatarFileSource::GravatarMirror.as_str(), "gravatar_mirror");
    assert_eq!("gravatar_mirror".parse::<AvatarFileSource>().unwrap(), AvatarFileSource::GravatarMirror);
  }
}
