ALTER TABLE gateway_projects ADD COLUMN attached_principal_id text;

ALTER TABLE gateway_projects ADD CONSTRAINT gateway_projects_attached_principal_fkey
    FOREIGN KEY (account_id, id, attached_principal_id)
    REFERENCES gateway_service_principals (account_id, project_id, id)
    ON DELETE SET NULL (attached_principal_id);

UPDATE gateway_projects p
SET attached_principal_id = (
    SELECT principal.id
    FROM gateway_service_principals principal
    WHERE (principal.account_id, principal.project_id) = (p.account_id, p.id)
      AND principal.revoked_at IS NULL
      AND EXISTS (
          SELECT 1 FROM gateway_project_keys key
          WHERE (key.account_id, key.project_id, key.principal_id) =
                (principal.account_id, principal.project_id, principal.id)
            AND key.revoked_at IS NULL AND key.expires_at > now()
      )
    ORDER BY (p.is_default AND lower(principal.name) = 'default') DESC,
             principal.created_at, principal.id
    LIMIT 1
);

ALTER TABLE gateway_requests
    ADD COLUMN billable_dispatch_at timestamptz,
    ADD COLUMN guarded_dispatch_at timestamptz;

-- A previous worker could have called the provider after claiming a paid request.
UPDATE gateway_requests
SET billable_dispatch_at = dispatched_at
WHERE adapter <> 'mock' AND estimated_usd_nanos > 0 AND dispatched_at IS NOT NULL;

CREATE FUNCTION gateway_request_can_cancel(request gateway_requests)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT num_nonnulls(($1).succeeded_at, ($1).canceled_at, ($1).failed_at, ($1).uncertain_at) = 0
      AND ($1).billable_dispatch_at IS NULL
      -- A worker from before this migration cannot set the billable boundary.
      AND (($1).dispatched_at IS NULL OR ($1).adapter = 'mock' OR ($1).estimated_usd_nanos = 0
           OR ($1).guarded_dispatch_at IS NOT DISTINCT FROM ($1).dispatched_at)
      AND (($1).adapter = 'mock' OR NOT EXISTS (
            SELECT 1 FROM gateway_usage_observations observation
            WHERE (observation.account_id, observation.request_id) = (($1).account_id, ($1).id)
              AND (observation.amount_usd_nanos > 0 OR observation.charge_quantity > 0)
              AND observation.source <> 'mock_provider'
      ))
$$;
