/// Lifecycle of a background response (`/v1/responses` with `background: true`): it starts
/// `queued`, moves to `in_progress`, and lands on a terminal state — poll until
/// [`is_terminal`](Self::is_terminal).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseStatus {
  Queued,
  InProgress,
  Completed,
  Failed,
  Cancelled,
  /// Finished but truncated (e.g. hit `max_output_tokens`) — partial output may exist.
  Incomplete,
  /// A status string this crate doesn't know yet (forward compat).
  Unknown(String),
}

impl ResponseStatus {
  pub(crate) fn from_wire(s: &str) -> Self {
    match s {
      "queued" => Self::Queued,
      "in_progress" => Self::InProgress,
      "completed" => Self::Completed,
      "failed" => Self::Failed,
      "cancelled" => Self::Cancelled,
      "incomplete" => Self::Incomplete,
      other => Self::Unknown(other.to_string()),
    }
  }

  /// Terminal = stop polling. `Unknown` counts as terminal so a new upstream state can
  /// never trap a poll loop forever.
  pub fn is_terminal(&self) -> bool {
    !matches!(self, Self::Queued | Self::InProgress)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_roundtrip_and_terminality() {
    assert_eq!(ResponseStatus::from_wire("queued"), ResponseStatus::Queued);
    assert!(!ResponseStatus::from_wire("in_progress").is_terminal());
    assert!(ResponseStatus::from_wire("completed").is_terminal());
    assert!(ResponseStatus::from_wire("failed").is_terminal());
    assert!(ResponseStatus::from_wire("someday-new-state").is_terminal());
  }
}
