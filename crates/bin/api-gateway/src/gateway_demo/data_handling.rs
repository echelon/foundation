use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const MAX_REGIONS: usize = 16;

/// Operator declaration for the full upstream path, including a downstream aggregator.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DataHandling {
    /// Every region in which this offer may process request content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_regions: Option<Vec<String>>,
    /// Upper bound on upstream storage of the request inputs and outputs, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_seconds: Option<u32>,
}

/// Hard limits to apply before ranking an upstream offer.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_regions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retention_seconds: Option<u32>,
}

#[derive(Clone, Copy)]
pub(super) enum DataMismatch {
    Region,
    Retention,
}

impl DataHandling {
    pub(super) fn normalize(&mut self) -> Result<(), &'static str> {
        if self.processing_regions.is_none() && self.retention_seconds.is_none() {
            return Err("an offer data handling declaration needs a region or retention bound");
        }
        normalize_regions(self.processing_regions.as_mut())
    }
}

impl DataRequirements {
    pub(super) fn normalize(&mut self) -> Result<(), &'static str> {
        normalize_regions(self.allowed_regions.as_mut())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.allowed_regions.is_none() && self.max_retention_seconds.is_none()
    }

    pub(super) fn intersect(&mut self, other: &Self) {
        if let Some(regions) = &other.allowed_regions {
            match &mut self.allowed_regions {
                Some(allowed) => allowed.retain(|region| regions.contains(region)),
                None => self.allowed_regions = Some(regions.clone()),
            }
        }
        if let Some(limit) = other.max_retention_seconds {
            self.max_retention_seconds = Some(self.max_retention_seconds.map_or(limit, |current| current.min(limit)));
        }
    }

    pub(super) fn mismatch(&self, data: Option<&DataHandling>) -> Option<DataMismatch> {
        if let Some(allowed) = &self.allowed_regions {
            let locations = data.and_then(|entry| entry.processing_regions.as_ref());
            if !locations.is_some_and(|regions| !regions.is_empty() && regions.iter().all(|region| allowed.contains(region))) {
                return Some(DataMismatch::Region);
            }
        }
        if let Some(limit) = self.max_retention_seconds {
            if !data.and_then(|entry| entry.retention_seconds).is_some_and(|seconds| seconds <= limit) {
                return Some(DataMismatch::Retention);
            }
        }
        None
    }
}

fn normalize_regions(regions: Option<&mut Vec<String>>) -> Result<(), &'static str> {
    let Some(regions) = regions else { return Ok(()) };
    if regions.is_empty() || regions.len() > MAX_REGIONS || regions.iter().any(|name| {
        !(2..=32).contains(&name.len())
            || !name.as_bytes()[0].is_ascii_lowercase()
            || name.split('-').any(|part| part.is_empty() || !part.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit()
            }))
    }) {
        return Err("use 1 to 16 lowercase region identifiers of 2 to 32 letters, digits, and hyphens");
    }
    regions.sort_unstable();
    let count = regions.len();
    regions.dedup();
    if regions.len() != count {
        return Err("region identifiers must be unique");
    }
    Ok(())
}
