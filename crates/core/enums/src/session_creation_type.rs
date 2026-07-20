use crate::text_enum::text_enum;

text_enum! {
  /// How a session came to exist (`sessions.created_via`) — login-frequency / session-age
  /// analysis fuel. Recorded once at creation, never updated.
  ///
  ///   - `signup`         — email/password signup that created the user (and their own org)
  ///   - `invited_signup` — signup that redeemed an invitation (membership, no own org)
  ///   - `login`          — email/password login (incl. signup-as-login on an existing account)
  ///   - `sso_login`      — SSO round-trip that matched an existing connection
  ///   - `sso_signup`     — SSO round-trip that created the user
  ///   - `impersonation`  — staff impersonation session (auth.md §6)
  pub enum SessionCreationType {
    Signup => "signup",
    InvitedSignup => "invited_signup",
    Login => "login",
    SsoLogin => "sso_login",
    SsoSignup => "sso_signup",
    Impersonation => "impersonation",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["signup", "invited_signup", "login", "sso_login", "sso_signup", "impersonation"];
    assert_eq!(SessionCreationType::ALL.len(), expect.len());
    for (v, s) in SessionCreationType::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<SessionCreationType>().unwrap(), *v);
    }
  }
}
