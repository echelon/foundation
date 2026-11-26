\getenv runtime_password TOKENCRAFT_RUNTIME_APP_PASSWORD
SELECT 'CREATE ROLE tokencraft_runtime LOGIN' WHERE NOT EXISTS (
    SELECT 1 FROM pg_roles WHERE rolname = 'tokencraft_runtime'
) \gexec
ALTER ROLE tokencraft_runtime PASSWORD :'runtime_password';
\unset runtime_password
