use serde::Deserialize;

/// One entry of a task's `outputs` array — a tagged union on `type`.
///
/// Which variants appear depends on what the task generated: video tasks emit
/// `video` outputs (plus `audio`/`voice`/`element` outputs for omni tasks that
/// produced them), image tasks emit `image` outputs, and so on.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TaskOutput {
  #[serde(rename = "video")]
  Video(VideoOutput),
  #[serde(rename = "image")]
  Image(ImageOutput),
  #[serde(rename = "audio")]
  Audio(AudioOutput),
  #[serde(rename = "voice")]
  Voice(VoiceOutput),
  #[serde(rename = "element")]
  Element(ElementOutput),
  /// Forward-compat: an output type this crate doesn't know yet.
  #[serde(other)]
  Unknown,
}

/// A generated video. URLs are hotlink-protected and purged after 30 days.
#[derive(Clone, Debug, Deserialize)]
pub struct VideoOutput {
  #[serde(default)]
  pub id: Option<String>,
  pub url: String,
  #[serde(default)]
  pub watermark_url: Option<String>,
  /// Duration in seconds — Kling sends it as a decimal string.
  #[serde(default)]
  pub duration: Option<String>,
}

/// A generated image. `group_id` only appears for grouped/series images.
#[derive(Clone, Debug, Deserialize)]
pub struct ImageOutput {
  pub url: String,
  #[serde(default)]
  pub watermark_url: Option<String>,
  #[serde(default)]
  pub group_id: Option<String>,
}

/// A generated audio clip, delivered in both MP3 and WAV.
#[derive(Clone, Debug, Deserialize)]
pub struct AudioOutput {
  #[serde(default)]
  pub id: Option<String>,
  #[serde(default)]
  pub mp3_url: Option<String>,
  #[serde(default)]
  pub wav_url: Option<String>,
  /// Durations in seconds — decimal strings.
  #[serde(default)]
  pub mp3_duration: Option<String>,
  #[serde(default)]
  pub wav_duration: Option<String>,
}

/// A custom voice attached to (or created by) the task. `owned_by` is
/// `"kling"` for the official library, otherwise a creator id. `status` is
/// `succeeded` or `deleted`.
#[derive(Clone, Debug, Deserialize)]
pub struct VoiceOutput {
  #[serde(default)]
  pub id: Option<String>,
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub owned_by: Option<String>,
  #[serde(default)]
  pub status: Option<String>,
}

/// A custom element attached to (or created by) the task. `element_type` is
/// `video_character_elements` or `multi_image_elements`; `status` is
/// `succeeded` or `deleted`.
#[derive(Clone, Debug, Deserialize)]
pub struct ElementOutput {
  #[serde(default)]
  pub id: Option<String>,
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub element_type: Option<String>,
  /// The element's reference materials. Query responses call this array
  /// `references`; the callback payload calls it `materials` — same content.
  #[serde(default, alias = "materials")]
  pub references: Vec<ElementReference>,
  #[serde(default)]
  pub owned_by: Option<String>,
  #[serde(default)]
  pub status: Option<String>,
  #[serde(default)]
  pub tags: Vec<ElementTag>,
}

/// One reference material of an element. Image roles are `frontal` vs
/// `reference` in query responses (`refer` in callbacks — same meaning);
/// video/voice roles are the fixed value `refer`.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ElementReference {
  #[serde(rename = "image")]
  Image {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    url: Option<String>,
  },
  #[serde(rename = "video")]
  Video {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    url: Option<String>,
  },
  #[serde(rename = "voice")]
  Voice {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
  },
  #[serde(other)]
  Unknown,
}

/// An element category tag (Hottest, Character, Animal, …).
#[derive(Clone, Debug, Deserialize)]
pub struct ElementTag {
  #[serde(default)]
  pub id: Option<i64>,
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn video_output_parses() {
    let json = r#"{"type":"video","id":"v_1","url":"https://cdn.example/v.mp4","watermark_url":"https://cdn.example/vw.mp4","duration":"5.0"}"#;
    let output: TaskOutput = serde_json::from_str(json).unwrap();
    match output {
      TaskOutput::Video(v) => {
        assert_eq!(v.url, "https://cdn.example/v.mp4");
        assert_eq!(v.duration.as_deref(), Some("5.0"));
      }
      other => panic!("expected video, got: {other:?}"),
    }
  }

  #[test]
  fn element_output_accepts_materials_alias_from_callbacks() {
    let json = r#"{
      "type": "element",
      "id": "e_1",
      "name": "Zhang",
      "element_type": "video_character_elements",
      "materials": [
        {"type": "image", "role": "frontal", "url": "https://cdn.example/f.png"},
        {"type": "voice", "role": "refer", "url": "https://cdn.example/v.mp3", "id": "voice_1", "name": "Zhang voice", "owned_by": "kling"}
      ],
      "owned_by": "12345",
      "status": "succeeded"
    }"#;
    let output: TaskOutput = serde_json::from_str(json).unwrap();
    match output {
      TaskOutput::Element(e) => {
        assert_eq!(e.references.len(), 2);
        assert!(matches!(&e.references[0], ElementReference::Image { role: Some(r), .. } if r == "frontal"));
      }
      other => panic!("expected element, got: {other:?}"),
    }
  }

  #[test]
  fn unknown_output_type_does_not_fail_the_parse() {
    let json = r#"[{"type":"hologram","url":"https://cdn.example/h"},{"type":"image","url":"https://cdn.example/i.png"}]"#;
    let outputs: Vec<TaskOutput> = serde_json::from_str(json).unwrap();
    assert!(matches!(outputs[0], TaskOutput::Unknown));
    assert!(matches!(outputs[1], TaskOutput::Image(_)));
  }
}
