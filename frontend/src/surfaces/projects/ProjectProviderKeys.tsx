import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { Badge, Button, Card, Input, Select } from "@/components/ui";
import { ApiError } from "@/lib/api";
import {
  credentialProviders, projectsApi, providerKeyNames,
  type ProviderKeyList, type ProviderKeyScope, type ProviderKeyTarget,
} from "./api";

type ProviderKeysProps = {
  scope: ProviderKeyScope;
  refresh?: number;
  onChange?: () => void;
};

export function ProjectProviderKeys({ scope, refresh, onChange }: ProviderKeysProps) {
  return credentialProviders.map((provider) =>
    <ProviderKeyPanel key={provider} scope={{ ...scope, provider }} refresh={refresh} onChange={onChange} />
  );
}

export function ConnectorProviderKeys({ scope, bindings, refresh }: {
  scope: { account_id: string; project_id?: string };
  bindings: string[];
  refresh?: number;
}) {
  const scopeId = useId();
  const [selected, setSelected] = useState(bindings[0]);
  const [level, setLevel] = useState<"project" | "organization">("project");
  const binding = bindings.includes(selected) ? selected : bindings[0];
  if (!binding) return null;

  return (
    <div className="space-y-3">
      {bindings.length > 1 && <label className="block max-w-md space-y-1 text-sm font-medium">
        Provider binding
        <Select value={binding} onChange={(event) => setSelected(event.target.value)}>
          {bindings.map((name) => <option key={name}>{name}</option>)}
        </Select>
      </label>}
      {scope.project_id && <fieldset className="min-w-0">
        <legend className="mb-1.5 text-sm font-medium">Credential scope</legend>
        <div className="flex flex-wrap gap-2">
          <label className="flex cursor-pointer items-center gap-2 rounded-md border border-border px-3 py-2 text-sm has-[:checked]:border-brand/60 has-[:checked]:bg-brand/10">
            <input type="radio" name={scopeId} className="accent-brand" checked={level === "project"}
              onChange={() => setLevel("project")} />
            Project override
          </label>
          <label className="flex cursor-pointer items-center gap-2 rounded-md border border-border px-3 py-2 text-sm has-[:checked]:border-brand/60 has-[:checked]:bg-brand/10">
            <input type="radio" name={scopeId} className="accent-brand" checked={level === "organization"}
              onChange={() => setLevel("organization")} />
            Organization default
          </label>
        </div>
      </fieldset>}
      <ProviderKeyPanel key={JSON.stringify([scope.account_id, scope.project_id, level, binding])}
        scope={{ account_id: scope.account_id, project_id: level === "project" ? scope.project_id : undefined, binding }}
        refresh={refresh} />
    </div>
  );
}

function credentialError(error: unknown, operation: "read" | "write") {
  if (error instanceof ApiError) {
    if (error.status === 0) return "The gateway is unreachable. Try again when it reconnects.";
    if ([401, 403, 404].includes(error.status)) return "Credential settings are unavailable for this account or project.";
    if (error.status === 400 && operation === "write") return "Check the key format and try again.";
    if (error.status === 409) {
      return error.code === "provider_credential_unavailable"
        ? "This binding is unavailable. Refresh the provider inventory."
        : "The credential changed or its default is unavailable. Review the latest version and try again.";
    }
  }
  return operation === "read" ? "Could not read credential versions." : "Could not update the credential. Try again.";
}

function ProviderKeyPanel({ scope, refresh = 0, onChange }: Omit<ProviderKeysProps, "scope"> & { scope: ProviderKeyTarget }) {
  const headingId = useId();
  const connector = "binding" in scope;
  const name = connector ? scope.binding : providerKeyNames[scope.provider];
  const organization = scope.project_id === undefined;
  const [keys, setKeys] = useState<ProviderKeyList>();
  const [revision, setRevision] = useState(0);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [loadError, setLoadError] = useState("");
  const [actionError, setActionError] = useState("");
  const [notice, setNotice] = useState("");
  const saving = useRef(false);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setLoadError("");
    projectsApi.providerKeys(scope, controller.signal).then((value) => {
      if (!controller.signal.aborted) setKeys(value);
    }).catch((failure: unknown) => {
      if (!controller.signal.aborted) setLoadError(credentialError(failure, "read"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [scope, refresh, revision]);

  const latest = keys?.versions[0];
  const fixture = !connector && keys?.mode !== "live";
  const fallback = keys?.organization_default;
  const inherits = !latest || latest.status === "inheriting";
  const disabled = pending || loading || !!loadError;

  async function change(action: () => Promise<void>, confirmation: string) {
    if (saving.current || disabled) return;
    saving.current = true;
    setPending(true);
    setActionError("");
    setNotice("");
    try {
      await action();
      setNotice(confirmation);
      onChange?.();
    } catch (failure) {
      setActionError(credentialError(failure, "write"));
    } finally {
      setLoading(true);
      setRevision((value) => value + 1);
      setPending(false);
      saving.current = false;
    }
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const key = new FormData(form).get("provider_key");
    if (typeof key !== "string" || disabled || saving.current) return;
    form.reset();
    await change(() => projectsApi.saveProviderKey(scope, latest?.version ?? 0, key),
      latest?.status === "active" ? "Key rotated." : "Key saved.");
  }

  if (!connector && !loadError && !actionError && (!keys || (!keys.enabled && !latest))) return null;

  return (
    <section aria-labelledby={headingId} className="min-w-0">
      <Card className="space-y-3 p-4 sm:p-5">
        <div>
          <h3 id={headingId} className="break-words font-display text-base font-semibold">
            {organization ? `Organization key · ${name}` : `Project key · ${name}`}
          </h3>
          <p className="mt-1 text-sm text-muted">
            {organization ? "Applies to projects that inherit this key." : "Choose a key for this project’s requests."}
            {latest?.status === "active" && " Queued requests keep their saved version after rotation."}
          </p>
        </div>
        {!organization && inherits && keys && (
          fallback?.status === "active"
            ? <p className="text-sm text-muted">Using organization key version {fallback.version} for new requests.</p>
            : <p className="text-sm text-muted">Set an organization default or save a project key to enable this binding.</p>
        )}
        {keys?.enabled && latest && !inherits && (latest.status === "revoked" || latest.status === "unavailable") &&
          <p className="text-sm text-muted">{fallback?.status === "active"
            ? "Save a replacement or use the organization default to resume new requests."
            : "Save a replacement to resume new requests."}</p>}
        {keys?.enabled && <form className="flex flex-wrap items-end gap-2.5" onSubmit={save}>
          <label className="block max-w-md flex-[1_1_13rem] space-y-1 text-sm font-medium">
            {connector ? "Provider API key" : `${name} ${fixture ? "fixture key" : "API key"}`}
            <Input name="provider_key" type="password" autoComplete="new-password" spellCheck={false}
              autoCapitalize="none" placeholder={fixture ? "fixture_…" : undefined}
              minLength={connector ? 16 : fixture ? 9 : 25} maxLength={fixture ? 64 : 128}
              required disabled={disabled} />
          </label>
          <Button className="rounded-md" type="submit" disabled={disabled}>
            {pending ? "Updating…" : latest?.status === "active" ? "Rotate key" : "Save key"}
          </Button>
        </form>}
        {keys?.enabled && latest && !inherits && fallback?.status === "active" &&
          <Button type="button" variant="outline" size="sm" className="rounded-md" disabled={disabled}
            onClick={() => void change(() => projectsApi.inheritProviderKey(scope, latest.version),
              "Project now uses the organization default.")}>
            Use organization default
          </Button>}
        {keys && !keys.enabled && <p className="text-sm text-muted">Credential storage is unavailable for this binding.</p>}
        {keys && keys.versions.length > 0 && <ul className="divide-y divide-border" aria-label="Provider key versions">
          {keys.versions.map((key) => <li key={key.version} className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm">
            <span>Version {key.version} <time className="ml-2 text-xs text-muted" dateTime={key.created_at}>{new Date(key.created_at).toLocaleDateString()}</time></span>
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone={key.status === "active" ? "green" : "muted"} className="capitalize">{key.status}</Badge>
              {(key.status === "active" || key.status === "retiring" || key.status === "unavailable") &&
                <Button type="button" size="sm" variant="outline" disabled={disabled} className="rounded-md" onClick={() => {
                  const effect = connector
                    ? "Requests still waiting to use this version will fail."
                    : "Requests still using it may need reconciliation.";
                  if (window.confirm(`Revoke ${organization ? "organization" : "project"} key version ${key.version} for ${name}? ${effect}`)) {
                    void change(() => projectsApi.revokeProviderKey(scope, key.version), "Key revoked.");
                  }
                }}>Revoke version {key.version}</Button>}
            </div>
          </li>)}
        </ul>}
        {loading && <p role="status" className="text-sm text-muted">Reading credential versions…</p>}
        {!loading && notice && <p role="status" className="text-sm text-muted">{notice}</p>}
        {loadError && <div className="flex flex-wrap items-center gap-2">
          <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{loadError}</p>
          <Button type="button" size="sm" variant="outline" className="rounded-md" disabled={loading}
            onClick={() => setRevision((value) => value + 1)}>Retry</Button>
        </div>}
        {actionError && <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{actionError}</p>}
      </Card>
    </section>
  );
}
