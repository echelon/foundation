import { useEffect, useState, type FormEvent } from "react";
import { Building2, Lock } from "lucide-react";
import { Button, Card, Input, Label } from "@/components/ui";
import { accountDisplayName, api, ApiError } from "@/lib/api";
import { useOrg } from "@/lib/org";

/**
 * Business settings (account-scoped). Owners and admins edit the business name (address,
 * billing email, etc. will join this page later); regular members see a frozen box. The
 * backend enforces the same rule — this page just mirrors it.
 *
 * Placeholder handling: until a business name is set, the field shows the generated
 * placeholder ("Adjective Noun") — but the moment it's focused it blanks, so nobody
 * accidentally SAVES the placeholder as their real name. Clearing the field (saving blank)
 * reverts to showing the placeholder.
 */
export default function Business() {
  const { current, loading, reload } = useOrg();
  const [value, setValue] = useState("");
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ tone: "ok" | "err"; text: string } | null>(null);

  const canEdit = current?.role === "owner" || current?.role === "admin";

  // (Re)fill from the scoped account: the set name, else the placeholder.
  useEffect(() => {
    if (current) {
      setValue(accountDisplayName(current));
      setDirty(false);
      setNotice(null);
    }
  }, [current]);

  if (loading || !current) {
    return <p className="text-muted">Loading…</p>;
  }

  const showingPlaceholder = current.business_name === null && !dirty;

  const onFocus = () => {
    // First interaction while the placeholder is displayed → blank the field.
    if (showingPlaceholder) {
      setValue("");
      setDirty(true);
    }
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!canEdit) return;
    setBusy(true);
    setNotice(null);
    try {
      const trimmed = value.trim();
      const r = await api.updateBusinessName(current.account_id, trimmed || null);
      setNotice({
        tone: "ok",
        text: r.business_name
          ? "Business name saved."
          : `Business name cleared — showing "${r.placeholder_business_name}" until one is set.`,
      });
      await reload(); // refresh the switcher + this form
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't save — try again." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Business settings</h1>
        <p className="mt-1 text-muted">
          How your organization appears across Foundation{canEdit ? "" : " — only owners and admins can change this"}.
        </p>
      </div>

      <Card className="p-6">
        <div className="flex items-center gap-2 font-display font-semibold">
          <Building2 className="h-4 w-4" /> Business name
        </div>
        {canEdit ? (
          <form className="mt-4 space-y-4" onSubmit={submit}>
            <div>
              <Label>Business name</Label>
              <Input
                value={value}
                onFocus={onFocus}
                onChange={(e) => {
                  setValue(e.target.value);
                  setDirty(true);
                }}
                className={showingPlaceholder ? "text-muted" : undefined}
                placeholder={current.placeholder_business_name}
              />
              <p className="mt-1.5 text-xs text-muted">
                {current.business_name === null ? (
                  <>
                    Not set yet — we're showing the generated placeholder{" "}
                    <span className="font-medium">{current.placeholder_business_name}</span> until you pick one.
                  </>
                ) : (
                  <>Leave blank and save to fall back to "{current.placeholder_business_name}".</>
                )}
              </p>
            </div>
            {notice && (
              <p
                className={
                  notice.tone === "ok"
                    ? "rounded-lg bg-emerald-500/10 px-3 py-2 text-sm text-emerald-600"
                    : "rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500"
                }
              >
                {notice.text}
              </p>
            )}
            <Button type="submit" disabled={busy}>
              {busy ? "Saving…" : "Save"}
            </Button>
          </form>
        ) : (
          <div className="mt-4">
            <Label>Business name</Label>
            {/* Frozen for regular members — the backend rejects their writes too. */}
            <div className="flex h-10 items-center justify-between rounded-lg border border-border bg-surface2 px-3 text-sm text-muted">
              <span>{accountDisplayName(current)}</span>
              <Lock className="h-3.5 w-3.5" />
            </div>
            <p className="mt-1.5 text-xs text-muted">Ask an owner or admin to change this.</p>
          </div>
        )}
      </Card>
    </div>
  );
}
