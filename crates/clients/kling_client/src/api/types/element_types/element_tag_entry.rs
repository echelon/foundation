use serde::{Deserialize, Serialize};

/// An element category tag. Outbound (create requests) only `tag_id` is sent;
/// documented tag ids:
///
/// | id      | Category | id      | Category |
/// | ------- | -------- | ------- | -------- |
/// | `o_101` | Hottest  | `o_105` | Costume  |
/// | `o_102` | Character| `o_106` | Scene    |
/// | `o_103` | Animal   | `o_107` | Effect   |
/// | `o_104` | Item     | `o_108` | Others   |
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ElementTagEntry {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tag_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
}

impl ElementTagEntry {
  pub fn from_tag_id(tag_id: impl Into<String>) -> Self {
    Self { tag_id: Some(tag_id.into()), name: None, description: None }
  }
}
