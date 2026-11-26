import { useEffect, useState, type FormEvent } from "react";
import { Button, Card, Input } from "@/components/ui";
import { errorMessage, gatewayRequest, type ProjectScope } from "@/lib/gateway";
import { dollarText, maxMonthlyBudget, parseDollarCap } from "@/lib/routing-policy";
import { formatNanoUsd, usd } from "@/surfaces/comparison/api";

type BudgetSetting = {
  client_ref: string;
  revision: number;
  limit_usd_nanos: number | null;
};
type BudgetPage = { budgets: BudgetSetting[]; next_after: string | null };
type Budget = BudgetSetting & { reserved_usd_nanos: string; resets_at: string };

const date = new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeZone: "UTC" });
const label = "block min-w-0 space-y-1 text-sm font-medium";
const errorClass = "text-sm text-rose-600 dark:text-rose-300";
const clientRefPattern = "[A-Za-z0-9][A-Za-z0-9_.:\\-]{0,63}";

export function ProjectClientBudgets({ scope }: { scope: ProjectScope }) {
  const [client, setClient] = useState("");
  const [ref, setRef] = useState("");
  const [after, setAfter] = useState<string>();
  const [page, setPage] = useState<BudgetPage>();
  const [issue, setIssue] = useState("");
  const [reload, setReload] = useState(0);
  const path = `/v1/projects/${encodeURIComponent(scope.project_id)}/client-budgets`;
  const organization = new URLSearchParams({ account_id: scope.account_id }).toString();

  useEffect(() => {
    const controller = new AbortController();
    const query = new URLSearchParams(organization);
    if (after) query.set("after", after);
    gatewayRequest<BudgetPage>(`${path}?${query}`, { signal: controller.signal }).then((result) => {
      setPage(result);
      setIssue("");
    }).catch((error) => {
      if (!controller.signal.aborted) setIssue(errorMessage(error));
    });
    return () => controller.abort();
  }, [after, organization, path, reload]);

  function select(value: string) {
    setRef(value);
    setClient(value);
  }

  function open(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (ref.trim()) select(ref.trim());
  }

  function paginate(cursor?: string) {
    setPage(undefined);
    setIssue("");
    setAfter(cursor);
  }

  return (
    <Card className="space-y-4 p-4 sm:p-5">
      <div>
        <h2 className="font-display text-xl font-semibold">Client monthly budgets</h2>
        <p className="mt-1 text-sm text-muted">Cap monthly estimates for an exact client reference across this project’s keys.</p>
      </div>
      <form onSubmit={open} className="flex flex-wrap items-end gap-3">
        <label className={`${label} w-full max-w-xs`}>Client reference
          <Input value={ref} onChange={(event) => setRef(event.target.value)}
            placeholder="e.g. customer-42" pattern={clientRefPattern} maxLength={64} required />
        </label>
        <Button type="submit" variant="outline">Open budget</Button>
      </form>
      {issue && <p role="alert" className={errorClass}>{issue}</p>}
      {page && !page.budgets.length && !after && <p className="text-sm text-muted">No client budgets configured.</p>}
      {!!page?.budgets.length && (
        <ul className="flex flex-wrap gap-2" aria-label="Configured client budgets">
          {page.budgets.map((budget) => (
            <li key={budget.client_ref} className="min-w-0">
              <Button type="button" size="sm" variant="outline" aria-pressed={client === budget.client_ref}
                className="h-auto max-w-full flex-wrap whitespace-normal break-all" onClick={() => select(budget.client_ref)}>
                {budget.client_ref} · {usd(budget.limit_usd_nanos)}
              </Button>
            </li>
          ))}
        </ul>
      )}
      {(after || page?.next_after) && (
        <div className="flex flex-wrap gap-2">
          {after && <Button type="button" size="sm" variant="outline" onClick={() => paginate()}>Back to first</Button>}
          {page?.next_after && <Button type="button" size="sm" variant="outline" onClick={() => paginate(page.next_after!)}>More budgets</Button>}
        </div>
      )}
      {client && <BudgetEditor key={client} client={client} path={`${path}/${encodeURIComponent(client)}?${organization}`}
        onSave={() => setReload((value) => value + 1)} />}
    </Card>
  );
}

function BudgetEditor({ client, path, onSave }: { client: string; path: string; onSave: () => void }) {
  const [budget, setBudget] = useState<Budget>();
  const [draft, setDraft] = useState("");
  const [issue, setIssue] = useState("");
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [reload, setReload] = useState(0);
  const [lifetime, setLifetime] = useState<AbortController>();

  useEffect(() => {
    const controller = new AbortController();
    setLifetime(controller);
    gatewayRequest<Budget>(path, { signal: controller.signal }).then((result) => {
      setBudget(result);
      setDraft(dollarText(result.limit_usd_nanos));
    }).catch((error) => {
      if (!controller.signal.aborted) setIssue(errorMessage(error));
    });
    return () => controller.abort();
  }, [path, reload]);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!budget || busy || !lifetime) return;
    const limit = parseDollarCap(draft, maxMonthlyBudget);
    setSaved(false);
    if (limit === undefined) {
      setIssue("Enter a budget from $0 to $1,000,000 with up to nine decimal places.");
      return;
    }
    setIssue("");
    setBusy(true);
    try {
      const result = await gatewayRequest<Budget>(path, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ expected_revision: budget.revision, limit_usd_nanos: limit }),
        signal: lifetime.signal,
      });
      if (lifetime.signal.aborted) return;
      setBudget(result);
      setDraft(dollarText(result.limit_usd_nanos));
      setSaved(true);
      onSave();
    } catch (error) {
      if (!lifetime.signal.aborted) setIssue(errorMessage(error));
    } finally {
      if (!lifetime.signal.aborted) setBusy(false);
    }
  }

  function refresh() {
    setIssue("");
    setSaved(false);
    setBudget(undefined);
    setReload((value) => value + 1);
  }

  return (
    <div className="space-y-3 border-t border-border pt-4">
      <h3 className="break-all text-sm font-semibold">{client}</h3>
      {!budget && !issue && <p role="status" className="text-sm text-muted">Loading budget…</p>}
      {budget && <>
        <p className="text-sm text-muted">
          {formatNanoUsd(budget.reserved_usd_nanos)} reserved{budget.limit_usd_nanos !== null && ` of ${usd(budget.limit_usd_nanos)}`}
          {" · Resets "}{date.format(new Date(budget.resets_at))} UTC
        </p>
        <form onSubmit={save} className="flex flex-wrap items-end gap-3">
          <label className={`${label} w-full max-w-xs`}>Monthly budget (USD)
            <Input inputMode="decimal" maxLength={17} value={draft} placeholder="Unlimited" disabled={busy}
              onChange={(event) => { setDraft(event.target.value); setSaved(false); }} />
          </label>
          <Button type="submit" disabled={busy}>{busy ? "Saving…" : "Save client budget"}</Button>
        </form>
        <p className="text-xs text-muted">A blank amount clears the limit. Zero blocks positive estimates.</p>
      </>}
      {issue && <div className="flex flex-wrap items-center gap-3">
        <p role="alert" className={errorClass}>{issue}</p>
        <Button type="button" size="sm" variant="outline" disabled={busy} onClick={refresh}>Reload budget</Button>
      </div>}
      {saved && <p role="status" className="text-sm text-muted">Client budget saved.</p>}
    </div>
  );
}
