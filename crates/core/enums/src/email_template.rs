use crate::text_enum::text_enum;

text_enum! {
  /// Transactional email templates (email.md §3). `Receipt` comes later.
  pub enum EmailTemplate {
    Invite => "invite",
    PasswordReset => "password_reset",
    EmailVerify => "email_verify",
    AutorefillFailed => "autorefill_failed",
    AccountSuspended => "account_suspended",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["invite", "password_reset", "email_verify", "autorefill_failed", "account_suspended"];
    for (v, s) in EmailTemplate::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
    }
  }
}
