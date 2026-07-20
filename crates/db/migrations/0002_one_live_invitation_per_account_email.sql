-- One live (unaccepted, unrevoked) invitation per (account, email) — accounts.md §4.
-- The service already revokes outstanding invitations before issuing a fresh one; this
-- partial unique index closes the race two concurrent creates would otherwise slip through
-- (db-queries/CLAUDE.md rule 5: integrity lives in the schema).
CREATE UNIQUE INDEX invitations_one_live_per_account_email
  ON invitations (account_id, email)
  WHERE accepted_at IS NULL AND revoked_at IS NULL;
