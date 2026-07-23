//! THE model registry (catalog-as-code, architecture.md §2). Models, their media type,
//! the dispatch provider, and the provider-specific model id we actually call.
//!
//! Launch set (this iteration): Nano Banana ×3 (Google), GPT Image ×4 (OpenAI),
//! Seedance 2.0 ×3 (Kinovi). Slugs are the canonical `models.md` slugs.

use serde::Serialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Mesh,
    Splat,
    Text,
}

/// Dispatch targets — who we actually call (enums.md `Provider`). Launch subset here.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Google Gemini image API — Nano Banana family.
    Google,
    /// OpenAI images API — GPT Image family.
    Openai,
    /// Kinovi (seedance2pro) — ByteDance Seedance video.
    Kinovi,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceUnit {
    PerImage,
    PerVideoSecond,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelSpec {
    /// Canonical catalog slug (`models.md`).
    pub slug: &'static str,
    pub display_name: &'static str,
    pub owner: &'static str,
    pub media_type: MediaType,
    /// Where we dispatch it.
    pub provider: Provider,
    /// The id the provider's API expects (e.g. `gemini-2.5-flash-image`, `gpt-image-1`, `seedance-20`).
    pub provider_model_id: &'static str,
    /// Granular tasks this model supports (`models.md` §11).
    pub modalities: &'static [&'static str],
    pub price_unit: PriceUnit,
    /// Headline unit price in nano-USD.
    pub price_nanos: i64,
}

/// The full launch catalog.
pub static MODELS: &[ModelSpec] = &[
    // ── Google · Nano Banana (image) ──
    ModelSpec {
        slug: "google_nano_banana",
        display_name: "Nano Banana",
        owner: "Google",
        media_type: MediaType::Image,
        provider: Provider::Google,
        provider_model_id: "gemini-2.5-flash-image",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 39_000_000, // $0.039 (flat per image)
    },
    ModelSpec {
        slug: "google_nano_banana_pro",
        display_name: "Nano Banana Pro",
        owner: "Google",
        media_type: MediaType::Image,
        provider: Provider::Google,
        provider_model_id: "gemini-3-pro-image",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 134_000_000, // $0.134 (1K/2K tier; 4K bills $0.24)
    },
    ModelSpec {
        slug: "google_nano_banana_2",
        display_name: "Nano Banana 2",
        owner: "Google",
        media_type: MediaType::Image,
        provider: Provider::Google,
        provider_model_id: "gemini-3.1-flash-image",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 67_000_000, // $0.067 (1K tier; 512px/2K/4K bill differently)
    },
    ModelSpec {
        slug: "google_nano_banana_2_lite",
        display_name: "Nano Banana 2 Lite",
        owner: "Google",
        media_type: MediaType::Image,
        provider: Provider::Google,
        provider_model_id: "gemini-3.1-flash-lite-image",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 33_600_000, // ~$0.0336 (1K only; token-priced)
    },
    // ── Google · Veo 3.1 (video) ──
    ModelSpec {
        slug: "google_veo_3_1",
        display_name: "Veo 3.1",
        owner: "Google",
        media_type: MediaType::Video,
        provider: Provider::Google,
        provider_model_id: "veo-3.1-generate-preview",
        modalities: &["text_to_video", "image_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 400_000_000, // $0.40/s at 720p/1080p ($0.60/s at 4K)
    },
    ModelSpec {
        slug: "google_veo_3_1_fast",
        display_name: "Veo 3.1 Fast",
        owner: "Google",
        media_type: MediaType::Video,
        provider: Provider::Google,
        provider_model_id: "veo-3.1-fast-generate-preview",
        modalities: &["text_to_video", "image_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 100_000_000, // $0.10/s at 720p ($0.12 1080p, $0.30 4K)
    },
    ModelSpec {
        slug: "google_veo_3_1_lite",
        display_name: "Veo 3.1 Lite",
        owner: "Google",
        media_type: MediaType::Video,
        provider: Provider::Google,
        provider_model_id: "veo-3.1-lite-generate-preview",
        modalities: &["text_to_video", "image_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 50_000_000, // $0.05/s at 720p ($0.08 1080p; no 4K)
    },
    // ── Google · Gemini Omni Flash (multimodal video, Interactions API) ──
    ModelSpec {
        slug: "google_omni_flash",
        display_name: "Gemini Omni Flash",
        owner: "Google",
        media_type: MediaType::Video,
        provider: Provider::Google,
        provider_model_id: "gemini-omni-flash-preview",
        modalities: &["text_to_video", "image_to_video", "video_edit"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 100_000_000, // token-billed ≈ $0.10/s of 720p — ESTIMATE; usage is authoritative
    },
    // ── OpenAI · GPT Image (image) ──
    ModelSpec {
        slug: "openai_gpt_image_1",
        display_name: "GPT Image 1",
        owner: "OpenAI",
        media_type: MediaType::Image,
        provider: Provider::Openai,
        provider_model_id: "gpt-image-1",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 40_000_000, // $0.040
    },
    ModelSpec {
        slug: "openai_gpt_image_1_mini",
        display_name: "GPT Image 1 Mini",
        owner: "OpenAI",
        media_type: MediaType::Image,
        provider: Provider::Openai,
        provider_model_id: "gpt-image-1-mini",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 11_000_000, // $0.011 (medium, 1024²)
    },
    ModelSpec {
        slug: "openai_gpt_image_1_5",
        display_name: "GPT Image 1.5",
        owner: "OpenAI",
        media_type: MediaType::Image,
        provider: Provider::Openai,
        provider_model_id: "gpt-image-1.5",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 30_000_000, // $0.030
    },
    ModelSpec {
        slug: "openai_gpt_image_2",
        display_name: "GPT Image 2",
        owner: "OpenAI",
        media_type: MediaType::Image,
        provider: Provider::Openai,
        provider_model_id: "gpt-image-2",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 60_000_000, // $0.060
    },
    // ── ByteDance · Seedance 2.0 via Kinovi (video) ──
    ModelSpec {
        slug: "bytedance_seedance_2",
        display_name: "Seedance 2.0",
        owner: "ByteDance",
        media_type: MediaType::Video,
        provider: Provider::Kinovi,
        provider_model_id: "seedance-20",
        modalities: &["text_to_video", "image_to_video", "reference_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 120_000_000, // $0.120 / sec
    },
    ModelSpec {
        slug: "bytedance_seedance_2_fast",
        display_name: "Seedance 2.0 Fast",
        owner: "ByteDance",
        media_type: MediaType::Video,
        provider: Provider::Kinovi,
        provider_model_id: "seedance2-fast",
        modalities: &["text_to_video", "image_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 60_000_000, // $0.060 / sec
    },
    ModelSpec {
        slug: "bytedance_seedance_2_mini",
        display_name: "Seedance 2.0 Mini",
        owner: "ByteDance",
        media_type: MediaType::Video,
        provider: Provider::Kinovi,
        provider_model_id: "seedance2.0-mini",
        modalities: &["text_to_video", "image_to_video"],
        price_unit: PriceUnit::PerVideoSecond,
        price_nanos: 30_000_000, // $0.030 / sec
    },
];

/// Look up a model by its canonical slug.
pub fn by_slug(slug: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for m in MODELS {
            assert!(seen.insert(m.slug), "duplicate slug: {}", m.slug);
        }
        assert_eq!(MODELS.len(), 15);
    }

    #[test]
    fn lookup_resolves_provider_id() {
        assert_eq!(by_slug("google_nano_banana").unwrap().provider_model_id, "gemini-2.5-flash-image");
        assert_eq!(by_slug("bytedance_seedance_2").unwrap().provider, Provider::Kinovi);
    }
}

impl MediaType {
    /// The wire/DB string for `jobs.media_type`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Mesh => "mesh",
            Self::Splat => "splat",
            Self::Text => "text",
        }
    }
}

impl Provider {
    /// The wire/DB string for `jobs.provider`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Openai => "openai",
            Self::Kinovi => "kinovi",
        }
    }
}
