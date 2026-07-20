//! THE model registry (catalog-as-code, architecture.md §2). Models, their media type,
//! the dispatch provider, and the provider-specific model id we actually call.
//!
//! Launch set (this iteration): Nano Banana ×3 (Google), GPT Image ×3 (OpenAI),
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
        price_nanos: 20_000_000, // $0.020
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
        price_nanos: 50_000_000, // $0.050
    },
    ModelSpec {
        slug: "google_nano_banana_v2",
        display_name: "Nano Banana 2",
        owner: "Google",
        media_type: MediaType::Image,
        provider: Provider::Google,
        provider_model_id: "gemini-3.1-flash-image",
        modalities: &["text_to_image", "image_edit"],
        price_unit: PriceUnit::PerImage,
        price_nanos: 40_000_000, // $0.040
    },
    // ── OpenAI · GPT Image (image) ──
    ModelSpec {
        slug: "openai_gpt_image_v1",
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
        slug: "openai_gpt_image_v1_5",
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
        slug: "openai_gpt_image_v2",
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
        slug: "bytedance_seedance_v2",
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
        slug: "bytedance_seedance_v2_fast",
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
        slug: "bytedance_seedance_v2_mini",
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
        assert_eq!(MODELS.len(), 9);
    }

    #[test]
    fn lookup_resolves_provider_id() {
        assert_eq!(by_slug("google_nano_banana").unwrap().provider_model_id, "gemini-2.5-flash-image");
        assert_eq!(by_slug("bytedance_seedance_v2").unwrap().provider, Provider::Kinovi);
    }
}
