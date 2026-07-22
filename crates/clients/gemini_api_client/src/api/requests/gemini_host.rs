/// Base URL for the Gemini API (AI Studio keys, `x-goog-api-key` header).
/// Model calls are `{GEMINI_API_BASE_URL}/models/{model}:{verb}`; operation
/// polls are `{GEMINI_API_BASE_URL}/{operation_name}`.
pub(crate) const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Base URL for Files API uploads (the resumable-upload surface lives under
/// `/upload/v1beta`, not `/v1beta`).
pub(crate) const GEMINI_UPLOAD_BASE_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta";

/// User-agent for every outbound request.
pub(crate) const GEMINI_USER_AGENT: &str = "foundation/gemini-api-client";
