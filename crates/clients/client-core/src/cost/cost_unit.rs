/// Billing unit for one priced component / rate-card row.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostUnit {
    /// Flat per request (e.g. Lyria's per-song price).
    PerRequest,
    /// Per output image.
    PerImage,
    /// Per second of output video.
    PerVideoSecond,
    /// Per 1M input tokens.
    PerMillionInputTokens,
    /// Per 1M output tokens (providers usually bill thinking tokens here).
    PerMillionOutputTokens,
}

impl CostUnit {
    /// Stable machine-readable name (frontends key labels/formatting off this).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PerRequest => "per_request",
            Self::PerImage => "per_image",
            Self::PerVideoSecond => "per_video_second",
            Self::PerMillionInputTokens => "per_million_tokens_in",
            Self::PerMillionOutputTokens => "per_million_tokens_out",
        }
    }

    /// How many units one "price" covers (1M for token units, 1 otherwise) —
    /// the divisor when turning `quantity × price` into a subtotal.
    pub(crate) fn units_per_price(&self) -> f64 {
        match self {
            Self::PerMillionInputTokens | Self::PerMillionOutputTokens => 1_000_000.0,
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_are_stable() {
        let all = [
            (CostUnit::PerRequest, "per_request"),
            (CostUnit::PerImage, "per_image"),
            (CostUnit::PerVideoSecond, "per_video_second"),
            (CostUnit::PerMillionInputTokens, "per_million_tokens_in"),
            (CostUnit::PerMillionOutputTokens, "per_million_tokens_out"),
        ];
        for (unit, expected) in all {
            assert_eq!(unit.as_str(), expected);
        }
    }

    #[test]
    fn token_units_price_per_million() {
        assert_eq!(CostUnit::PerMillionInputTokens.units_per_price(), 1_000_000.0);
        assert_eq!(CostUnit::PerImage.units_per_price(), 1.0);
    }
}
