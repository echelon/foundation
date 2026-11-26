import { useEffect, useState } from "react";

const time = new Intl.DateTimeFormat(undefined, {
  hour: "numeric", minute: "2-digit", second: "2-digit", timeZoneName: "short",
});

export function quoteExpired(value?: string) {
  return value !== undefined && Date.parse(value) <= Date.now();
}

export function useDeadlineClock(deadlines: (string | undefined)[]) {
  const [, setTick] = useState(0);
  const now = Date.now();
  const next = Math.min(...deadlines.map(value => Date.parse(value ?? "")).filter(time => time > now));
  useEffect(() => {
    if (!Number.isFinite(next)) return;
    const timeout = window.setTimeout(() => setTick(tick => tick + 1), Math.max(0, next - Date.now()) + 1);
    return () => window.clearTimeout(timeout);
  }, [next]);
  return now;
}

export function QuoteExpiry({ expiresAt }: { expiresAt?: string }) {
  const now = useDeadlineClock([expiresAt]);
  if (!expiresAt) return null;
  if (Date.parse(expiresAt) <= now) {
    return <span>Estimate expired. Refresh for a new request.</span>;
  }
  return <span>Estimate valid until <time dateTime={expiresAt}>{time.format(new Date(expiresAt))}</time>.</span>;
}
