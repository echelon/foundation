import { useEffect, useId, useState, type FormEvent } from "react";
import { KeyRound, Plus } from "lucide-react";
import { Badge, Button, Checkbox, Input } from "@/components/ui";
import { errorMessage, type ProjectScope } from "@/lib/gateway";
import { keyScopes, projectsApi, type Credentials, type IssuedKey, type KeyScope } from "./api";
import { OneTimeSecret } from "./OneTimeSecret";

const lifetime = 90;
const date = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
const scopeOptions = Object.keys(keyScopes) as KeyScope[];
const requestScopes: KeyScope[] = ["requests:submit", "requests:read"];

export function ProjectCredentials({ scope, isDefault }: { scope: ProjectScope; isDefault: boolean }) {
  const editorId = useId();
  const [creating, setCreating] = useState(false);
  const [data, setData] = useState<Credentials>();
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [issued, setIssued] = useState<{ key: IssuedKey; name: string }>();
  const { account_id, project_id } = scope;

  useEffect(() => {
    const controller = new AbortController();
    projectsApi.credentials({ account_id, project_id }, controller.signal).then((value) => {
      if (!controller.signal.aborted) setData(value);
    }).catch((failure) => {
      if (!controller.signal.aborted) setError(errorMessage(failure));
    });
    return () => controller.abort();
  }, [account_id, project_id, revision]);

  async function change(action: () => Promise<void | IssuedKey>, message: string, issuedName?: string) {
    if (busy || issued) return false;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const key = await action();
      if (key && issuedName) setIssued({ key, name: issuedName });
      setNotice(message);
      setRevision((value) => value + 1);
      return true;
    } catch (failure) {
      setError(errorMessage(failure));
      return false;
    } finally {
      setBusy(false);
    }
  }

  const disabled = busy || Boolean(issued);
  const attached = data?.principals.find((principal) => principal.id === data.attached_principal_id);
  const liveKeys = new Set(data?.keys.filter((key) => !key.revoked_at && Date.parse(key.expires_at) > Date.now())
    .map((key) => key.principal_id));
  const setupDefault = data && !attached && isDefault && !data.principals.some((principal) => !principal.revoked_at);
  const defaultKey = () => change(() => projectsApi.createPrincipal(scope, {
    name: "Default",
    scopes: requestScopes,
    client_ref: null,
    expires_in_days: lifetime,
    bootstrap_default: true,
    attach: true,
  }), "Default is attached to this project.", "Default");

  return (
    <div className="min-w-0 space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-surface2/45 p-3.5 sm:p-4">
        <div className="flex min-w-0 items-start gap-3">
          <span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-border bg-surface text-brand">
            <KeyRound className="h-4 w-4" aria-hidden />
          </span>
          <div className="min-w-0">
            <p className="text-xs font-medium text-muted">Attached key</p>
            <p className="mt-0.5 break-words font-semibold">{attached?.name ?? (setupDefault ? "Default" : data ? "No key attached" : "Loading…")}</p>
            {attached && <p className="mt-1 text-xs text-muted">{liveKeys.has(attached.id) ? attached.scopes.map((permission) => keyScopes[permission]).join(" · ") : "No active token. Rotate this key or attach another one."}</p>}
            {setupDefault && <p className="mt-1 text-xs text-muted">Issue the first key for the Default project.</p>}
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          {setupDefault && <Button type="button" size="sm" disabled={disabled} onClick={() => void defaultKey()}>Create Default key</Button>}
          <Button type="button" variant={setupDefault ? "ghost" : "outline"} size="sm" disabled={disabled || !data}
            aria-expanded={creating} aria-controls={editorId} onClick={() => setCreating((value) => !value)}>
            {!creating && <Plus size={15} aria-hidden />}
            {creating ? "Close form" : setupDefault ? "Create custom key" : "Create key"}
          </Button>
        </div>
      </div>
      <div id={editorId} hidden={!creating}>
        {creating && <CreateKey disabled={disabled} onCreate={async (input) => {
          if (await change(() => projectsApi.createPrincipal(scope, input),
            input.attach ? `${input.name} is attached to this project.` : `${input.name} was created.`, input.name)) {
            setCreating(false);
          }
        }} />}
      </div>
      {issued && <OneTimeSecret key={issued.key.id} label={`${issued.name} token`} noun="token" value={issued.key.token}
        metadata={`Expires ${date.format(new Date(issued.key.expires_at))}`} dismiss={() => setIssued(undefined)} />}
      {notice && <p role="status" className="text-sm text-muted">{notice}</p>}
      {error && <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{error}{" "}
        <button type="button" className="underline" onClick={() => { setError(""); setRevision((value) => value + 1); }}>Refresh keys</button>
      </p>}
      {data && data.principals.length > 0 && (
        <ul className="divide-y divide-border rounded-xl border border-border">
          {data.principals.map((principal) => {
            const selected = attached?.id === principal.id;
            const tokens = data.keys.filter((key) => key.principal_id === principal.id);
            return (
              <li key={principal.id} className="space-y-3 p-3.5 sm:p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="break-words text-sm font-semibold">{principal.name}</p>
                      {selected && <Badge tone="brand">Attached</Badge>}
                      {principal.revoked_at && <Badge>Revoked</Badge>}
                    </div>
                    {principal.client_ref && <p className="mt-1 break-words text-xs text-muted">Client {principal.client_ref}</p>}
                    {!selected && <p className="mt-1 break-words text-xs text-muted">{principal.scopes.map((permission) => keyScopes[permission]).join(" · ")}</p>}
                  </div>
                  {!principal.revoked_at && <div className="flex flex-wrap gap-1.5">
                    {!selected && liveKeys.has(principal.id) && <Button type="button" variant="outline" size="sm" disabled={disabled}
                      onClick={() => void change(() => projectsApi.attachKey(scope, principal.id), `${principal.name} is attached to this project.`)}>
                      Attach key
                    </Button>}
                    <Button type="button" variant="ghost" size="sm" disabled={disabled}
                      onClick={() => void change(() => projectsApi.rotateKey(scope, principal.id, lifetime), "New token issued.", principal.name)}>
                      Rotate
                    </Button>
                  </div>}
                </div>
                <details className="rounded-lg bg-surface2/45 p-2.5 text-xs text-muted">
                  <summary className="cursor-pointer font-medium">Token history and revocation</summary>
                  <ul className="mt-2 divide-y divide-border">
                    {tokens.map((key) => {
                      const active = !principal.revoked_at && !key.revoked_at && Date.parse(key.expires_at) > Date.now();
                      return (
                        <li key={key.id} className="flex flex-wrap items-center justify-between gap-2 py-1.5">
                          <span>{key.revoked_at ? "Revoked" : active ? "Expires" : "Expired"} {date.format(new Date(key.revoked_at ?? key.expires_at))}</span>
                          {active && <Button type="button" variant="ghost" size="sm" disabled={disabled}
                            onClick={() => {
                              if (window.confirm(`Revoke this token for ${principal.name}?`)) {
                                void change(() => projectsApi.revokeKey(scope, key.id), "Token revoked.");
                              }
                            }}>Revoke token</Button>}
                        </li>
                      );
                    })}
                  </ul>
                  {!principal.revoked_at && <Button type="button" variant="ghost" size="sm" disabled={disabled}
                    onClick={() => {
                      if (window.confirm(`Revoke every token for ${principal.name}?`)) {
                        void change(() => projectsApi.revokePrincipal(scope, principal.id), `${principal.name} was revoked.`);
                      }
                    }}>Revoke all tokens</Button>}
                </details>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

type KeyInput = Parameters<typeof projectsApi.createPrincipal>[1];

function CreateKey({ disabled, onCreate }: { disabled: boolean; onCreate: (input: KeyInput) => Promise<void> }) {
  const [name, setName] = useState("");
  const [client, setClient] = useState("");
  const [expires, setExpires] = useState(String(lifetime));
  const [scopes, setScopes] = useState<KeyScope[]>(requestScopes);
  const [attach, setAttach] = useState(true);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || !scopes.length || disabled) return;
    await onCreate({ name: name.trim(), scopes, client_ref: client.trim() || null, expires_in_days: Number(expires), attach });
  }

  return (
    <form onSubmit={submit} className="rounded-xl border border-border bg-surface p-3.5 sm:p-4">
      <fieldset disabled={disabled} className="min-w-0 space-y-4">
        <div className="grid gap-3 sm:grid-cols-3">
          <label className="block min-w-0 space-y-1 text-sm font-medium">
            Key name
            <Input value={name} required maxLength={80} onChange={(event) => setName(event.target.value)} placeholder="e.g. Video worker" />
          </label>
          <label className="block min-w-0 space-y-1 text-sm font-medium">
            Client reference
            <Input value={client} maxLength={64} onChange={(event) => setClient(event.target.value)} placeholder="Optional" />
          </label>
          <label className="block min-w-0 space-y-1 text-sm font-medium">
            Expiration in days
            <Input type="number" min={1} max={365} required value={expires} onChange={(event) => setExpires(event.target.value)} />
          </label>
        </div>
        <fieldset>
          <legend className="mb-2 text-sm font-medium">Permissions</legend>
          <div className="grid gap-2 sm:grid-cols-2">
            {scopeOptions.map((permission) => (
              <label key={permission} className="flex min-w-0 cursor-pointer items-start gap-2.5 rounded-lg border border-border p-2.5 text-sm has-[:checked]:bg-brand/5">
                <Checkbox className="mt-0.5" checked={scopes.includes(permission)} onChange={(event) => {
                  setScopes((items) => event.target.checked ? [...items, permission] : items.filter((item) => item !== permission));
                }} />
                <span>{keyScopes[permission]}
                  {permission === "requests:cancel" && <span className="block text-xs text-muted">Available before paid inference starts</span>}
                </span>
              </label>
            ))}
          </div>
        </fieldset>
        <label className="flex cursor-pointer items-center gap-2.5 text-sm">
          <Checkbox checked={attach} onChange={(event) => setAttach(event.target.checked)} />
          Attach as project key
        </label>
        <Button type="submit" disabled={!name.trim() || !scopes.length}>Issue key</Button>
      </fieldset>
    </form>
  );
}
