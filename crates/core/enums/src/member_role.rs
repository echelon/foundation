use crate::text_enum::text_enum;

text_enum! {
  /// Account membership role (auth.md §4) — gates every account-scoped authorization.
  /// NEVER compare bare role strings; use this enum.
  pub enum MemberRole {
    Owner => "owner",
    Admin => "admin",
    Member => "member",
  }
}

impl MemberRole {
  /// Roles allowed to administer the account (everything except plain membership).
  pub const ADMINS: &'static [MemberRole] = &[MemberRole::Owner, MemberRole::Admin];
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["owner", "admin", "member"];
    assert_eq!(MemberRole::ALL.len(), expect.len());
    for (v, s) in MemberRole::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<MemberRole>().unwrap(), *v);
    }
  }
}
