use serde::ser::SerializeMap;
use serde::Serialize;

/// A tool made available to the model. Built-ins are Google-hosted; `Function`
/// declares a callable the CALLER executes when the model requests it.
#[derive(Clone, Debug)]
pub enum GeminiChatTool {
  /// Google Search grounding.
  GoogleSearch,
  /// Server-side Python sandbox.
  CodeExecution,
  /// Let the model fetch URLs mentioned in the prompt.
  UrlContext,
  /// A caller-executed function (JSON-Schema parameters).
  Function { name: String, description: String, parameters: serde_json::Value },
}

impl Serialize for GeminiChatTool {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(None)?;
    match self {
      Self::GoogleSearch => map.serialize_entry("type", "google_search")?,
      Self::CodeExecution => map.serialize_entry("type", "code_execution")?,
      Self::UrlContext => map.serialize_entry("type", "url_context")?,
      Self::Function { name, description, parameters } => {
        map.serialize_entry("type", "function")?;
        map.serialize_entry("name", name)?;
        map.serialize_entry("description", description)?;
        map.serialize_entry("parameters", parameters)?;
      }
    }
    map.end()
  }
}
