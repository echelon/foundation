use crate::text_enum::text_enum;

text_enum! {
  /// Why a job failed (api_overview.md §2) — on the job, not the HTTP reply.
  pub enum FailureCode {
    ContentPolicyInput => "content_policy_input",
    ContentPolicyOutput => "content_policy_output",
    FaceOrPersonBlocked => "face_or_person_blocked",
    CopyrightBlocked => "copyright_blocked",
    InvalidInput => "invalid_input",
    InputFetchFailed => "input_fetch_failed",
    InputUnsupported => "input_unsupported",
    ProviderError => "provider_error",
    ProviderTimeout => "provider_timeout",
    ProviderCapacity => "provider_capacity",
    ProviderDeprecated => "provider_deprecated",
    Canceled => "canceled",
    Expired => "expired",
    OutputIngestFailed => "output_ingest_failed",
    DryRun => "dry_run",
    Internal => "internal",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["content_policy_input", "content_policy_output", "face_or_person_blocked", "copyright_blocked", "invalid_input", "input_fetch_failed", "input_unsupported", "provider_error", "provider_timeout", "provider_capacity", "provider_deprecated", "canceled", "expired", "output_ingest_failed", "dry_run", "internal"];
    assert_eq!(FailureCode::ALL.len(), expect.len());
    for (v, s) in FailureCode::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<FailureCode>().unwrap(), *v);
    }
  }
}
