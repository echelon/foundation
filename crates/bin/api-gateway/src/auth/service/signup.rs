//! Email/password signup incl. signup-as-login and invite redemption (accounts.md §3A, §4).


use db_queries::{
  accounts, members, users,
};
use enums::SessionCreationType;

use crate::auth::email::{canonicalize, canonicalize_clean};
use crate::auth::password::{hash_password, verify_password};
use crate::auth::token::JwtKeys;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// Everything `signup` needs beyond connections (named fields — the email/password/name run
/// of `&str`s is exactly the transposition hazard the args-struct rule exists for).
pub struct SignupArgs<'a> {
  pub email: &'a str,
  pub password: &'a str,
  pub display_name: Option<&'a str>,
  /// An invitation code or magic-link token to redeem during signup (accounts.md §4 flow 1).
  /// When set, the email is LOCKED to the invite's email, and the new user gets a membership
  /// in the inviting account INSTEAD of a personal organization (invites never create
  /// accounts — only memberships).
  pub invitation: Option<&'a str>,
}

/// Email + password signup (accounts.md §3A), including the signup-as-login rule and the
/// invite-redemption flows (accounts.md §4).
pub async fn signup(pool: &db::PgPool, jwt: &JwtKeys, args: SignupArgs<'_>, meta: &ReqMeta) -> ApiResult<AuthOutcome> {
  if args.password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  let email_canon = canonicalize(args.email);

  // Resolve the invitation up front so a dead invite fails before any user is created, and
  // enforce the email lock (the join page pre-fills and locks the field; the API is the
  // actual gate).
  let invitation = match args.invitation {
    Some(code_or_token) => {
      let inv = resolve_invitation(pool, code_or_token).await?;
      if inv.email != email_canon {
        return Err(ApiError::BadRequest("this invitation was issued to a different email address".into()));
      }
      Some(inv)
    }
    None => None,
  };

  // Existing user? signup-as-login when the password is correct; otherwise route to login.
  if let Some(user) = users::find_user_by_email(pool, &email_canon).await? {
    match &user.password_hash {
      Some(hash) if verify_password(args.password, hash) => {
        // Flow 2 (accounts.md §4): an existing user redeeming an invite — accept, then log in.
        if let Some(inv) = &invitation {
          let mut tx = pool.begin().await?;
          accept_open_invitation(&mut tx, inv, &user.id, meta.ip.as_deref()).await?;
          tx.commit().await?;
        }
        return start_session(pool, jwt, user, None, meta, SessionCreationType::Login).await;
      }
      _ => {
        return Err(ApiError::Conflict(
          "an account with this email already exists — log in instead".into(),
        ));
      }
    }
  }

  // Genuinely new — create everything atomically.
  let hash = hash_password(args.password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;

  let user = users::create_user(
    &mut *tx,
    users::NewUser {
      email: email_canon,
      email_clean: canonicalize_clean(args.email),
      email_original: args.email.trim().to_string(),
      password_hash: Some(hash),
      display_name: args.display_name.map(str::to_string),
      email_verified_at: None,
    },
  )
  .await?;
  match &invitation {
    // Normal signup: the user's own organization, with them as owner.
    None => {
      let account = accounts::create_account(
        &mut *tx,
        accounts::NewAccount { name: None, placeholder_business_name: &crate::business_names::random_business_name() },
      )
      .await?;
      members::create_membership(&mut *tx, &account.id, &user.id, enums::MemberRole::Owner).await?;
      provision_new_user(&mut tx, &user, &account, meta).await?;
    }
    // Invited signup (flow 1): membership in the INVITING account only — no personal org.
    // The accept writes the member_added/invitation_accepted audit rows; account_created
    // would be a lie here, so provisioning is just the avatar.
    Some(inv) => {
      create_default_avatar(&mut tx, &user).await?;
      accept_open_invitation(&mut tx, inv, &user.id, meta.ip.as_deref()).await?;
    }
  }
  let created_via =
    if invitation.is_some() { SessionCreationType::InvitedSignup } else { SessionCreationType::Signup };
  let session = new_session(&mut tx, &user.id, meta, created_via).await?;

  tx.commit().await?;

  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}
