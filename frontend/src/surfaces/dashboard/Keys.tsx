import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Check, Copy, Plus, Trash2 } from "lucide-react";
import { Button, Card, Input, Label } from "@/components/ui";
import { UserAvatar } from "@/components/UserAvatar";
import { api, ApiError, type ApiAccountUser, type ApiKeyItem } from "@/lib/api";
import { useOrg } from "@/lib/org";

/**
 * API keys, scoped to the org in the header switcher. Any member may create and delete
 * (auth.md §4, revised 2026-07-21). A key's full secret is shown exactly once, at creation —
 * afterwards only the identifying prefix survives.
 */
export default function Keys() {
  const { current, loading } = useOrg();
  const [keys, setKeys] = useState<ApiKeyItem[] | null>(null);
  const [creators, setCreators] = useState<Record<string, ApiAccountUser>>({});
  const [error, setError] = useState<string | null>(null);

  // One-time secret from the most recent creation (cleared on navigation/reload — by design).
  const [minted, setMinted] = useState<{ title: string; secret: string } | null>(null);

  const [showForm, setShowForm] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const accountId = current?.account_id ?? null;

  const load = useCallback(async () => {
    if (!accountId) return;
    try {
      const r = await api.listApiKeys(accountId);
      setKeys(r.api_keys);
      setError(null);
      // Resolve creators for display (helper, not a gate — failures just show no portrait).
      const ids = [...new Set(r.api_keys.map((k) => k.created_by_user_id))];
      if (ids.length > 0) {
        api
          .accountUsers(accountId, ids)
          .then((res) => {
            const map: Record<string, ApiAccountUser> = {};
            for (const u of res.users) map[u.user_id] = u;
            setCreators(map);
          })
          .catch(() => {});
      }
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Couldn't load API keys.");
    }
  }, [accountId]);

  useEffect(() => {
    setKeys(null);
    setMinted(null);
    setShowForm(false);
    void load();
  }, [load]);

  if (loading || !current) return null;

  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (!accountId || !title.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const r = await api.createApiKey(accountId, title.trim(), description.trim() || undefined);
      setMinted({ title: r.api_key.title, secret: r.secret });
      setTitle("");
      setDescription("");
      setShowForm(false);
      await load();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Couldn't create the key.");
    } finally {
      setBusy(false);
    }
  };

  const remove = async (k: ApiKeyItem) => {
    if (!accountId) return;
    if (!window.confirm(`Delete "${k.title}"? Anything using it stops working immediately.`)) return;
    setDeleting(k.id);
    try {
      await api.deleteApiKey(accountId, k.id);
      await load();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Couldn't delete the key.");
    } finally {
      setDeleting(null);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl font-bold tracking-tight">API keys</h1>
          <p className="mt-1 text-muted">One secret authenticates every endpoint. Rotate anytime.</p>
        </div>
        <Button onClick={() => setShowForm((v) => !v)}>
          <Plus className="h-4 w-4" /> Create key
        </Button>
      </div>

      <Card className="flex items-start gap-3 border-amber-500/30 bg-amber-500/[0.06] p-4">
        <p className="text-sm text-muted">
          Treat keys like passwords. Keys look like{" "}
          <code className="rounded bg-surface2 px-1.5 py-0.5">foundation_api_…</code> and are shown in full
          only once, at creation — copy them somewhere safe.
        </p>
      </Card>

      {showForm && (
        <Card className="p-5">
          <form className="space-y-4" onSubmit={create}>
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <Label>Name</Label>
                <Input
                  placeholder="Production backend"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  required
                  autoFocus
                />
              </div>
              <div>
                <Label>Description (optional)</Label>
                <Input
                  placeholder="What uses this key?"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </div>
            </div>
            <div className="flex gap-2">
              <Button type="submit" disabled={busy || !title.trim()}>
                {busy ? "Creating…" : "Create key"}
              </Button>
              <Button type="button" variant="ghost" onClick={() => setShowForm(false)}>
                Cancel
              </Button>
            </div>
          </form>
        </Card>
      )}

      {minted && <MintedSecret title={minted.title} secret={minted.secret} onDismiss={() => setMinted(null)} />}

      {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}

      <Card className="overflow-hidden">
        <div className="scrollbar-thin overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="border-b border-border text-left text-muted">
              <tr>
                <th className="px-5 py-3 font-medium">Name</th>
                <th className="px-5 py-3 font-medium">Key</th>
                <th className="px-5 py-3 font-medium">Created by</th>
                <th className="px-5 py-3 font-medium">Created</th>
                <th className="px-5 py-3 font-medium">Last used</th>
                <th className="px-5 py-3"></th>
              </tr>
            </thead>
            <tbody>
              {keys === null ? (
                <tr>
                  <td colSpan={6} className="px-5 py-4 text-muted">
                    Loading…
                  </td>
                </tr>
              ) : keys.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-5 py-4 text-muted">
                    No API keys yet — create one to start calling the API.
                  </td>
                </tr>
              ) : (
                keys.map((k) => {
                  const creator = creators[k.created_by_user_id];
                  return (
                    <tr key={k.id} className="border-b border-border/60 last:border-0">
                      <td className="px-5 py-3">
                        <div className="font-medium text-fg">{k.title}</div>
                        {k.description && <div className="text-xs text-muted">{k.description}</div>}
                      </td>
                      <td className="px-5 py-3">
                        <code className="font-mono text-[13px] text-muted">{k.key_prefix}••••••••</code>
                      </td>
                      <td className="px-5 py-3">
                        {creator ? (
                          <div className="flex items-center gap-2">
                            <UserAvatar user={creator} className="h-6 w-6 shrink-0 text-[10px]" />
                            <span className="text-muted">{creator.display_name ?? creator.email}</span>
                          </div>
                        ) : (
                          <span className="text-xs text-muted">—</span>
                        )}
                      </td>
                      <td className="px-5 py-3 text-muted">{new Date(k.created_at).toLocaleDateString()}</td>
                      {/* Last used: date AND time. */}
                      <td className="px-5 py-3 text-muted">
                        {k.last_used_at ? new Date(k.last_used_at).toLocaleString() : "never"}
                      </td>
                      <td className="px-5 py-3 text-right">
                        <button
                          onClick={() => void remove(k)}
                          disabled={deleting !== null}
                          title="Delete key"
                          className="text-muted transition hover:text-rose-500 disabled:opacity-50"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

/** The one-time secret reveal — visible until dismissed or the page changes. */
function MintedSecret({ title, secret, onDismiss }: { title: string; secret: string; onDismiss: () => void }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(secret);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard unavailable — the user can still select the text
    }
  };

  return (
    <Card className="border-emerald-500/30 bg-emerald-500/[0.06] p-5">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="font-medium text-fg">"{title}" created — copy the key now.</p>
          <p className="mt-0.5 text-xs text-muted">
            This is the only time it will be shown. If you lose it, delete the key and create a new one.
          </p>
          <code className="mt-3 block select-all break-all rounded-lg bg-surface2 px-3 py-2 font-mono text-[13px] text-fg">
            {secret}
          </code>
        </div>
        <div className="flex shrink-0 flex-col gap-2">
          <Button size="sm" onClick={() => void copy()}>
            {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? "Copied" : "Copy"}
          </Button>
          <Button size="sm" variant="ghost" onClick={onDismiss}>
            Done
          </Button>
        </div>
      </div>
    </Card>
  );
}
