import { useEffect, useRef, useState } from "react";
import { CloudOff, Megaphone } from "lucide-react";
import { API_REQUEST_FAILED_EVENT, fetchApiStatus, type ApiStatus } from "@/lib/api-status";

/** Healthy cadence — lazy; this is a helper, not a heartbeat. */
const HEALTHY_POLL_MS = 30_000;
/** During an outage, re-check every 6–10s (jittered so a fleet of tabs doesn't thundering-herd
 * the recovering server). */
const OUTAGE_POLL_MIN_MS = 6_000;
const OUTAGE_POLL_JITTER_MS = 4_000;
/** First check is deferred so page load never waits on it. */
const FIRST_CHECK_MS = 2_000;
/** Failure-triggered rechecks are rate-limited to this. */
const MIN_RECHECK_GAP_MS = 10_000;

/**
 * A slim banner that appears only when something is wrong: the API is unreachable, or an
 * operator status message is set (`/v1/api_status`). Mounted on the auth pages and the
 * dashboard. Renders nothing in the happy path; every failure of the probe itself is
 * swallowed — this can only ever ADD information, never break a page.
 */
export function ApiStatusBanner() {
  const [status, setStatus] = useState<ApiStatus>({ reachable: true, message: null });
  const lastCheck = useRef(0);
  const inFlight = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;

    const nextDelay = (s: ApiStatus) =>
      s.reachable ? HEALTHY_POLL_MS : OUTAGE_POLL_MIN_MS + Math.random() * OUTAGE_POLL_JITTER_MS;

    // Self-scheduling chain (not setInterval) so the cadence adapts: tight while down,
    // relaxed once recovered.
    const schedule = (delay: number) => {
      if (cancelled) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void check(true), delay);
    };

    const check = async (force: boolean) => {
      if (cancelled || inFlight.current) return;
      if (!force && Date.now() - lastCheck.current < MIN_RECHECK_GAP_MS) return;
      if (document.hidden) {
        // Nobody's looking — skip the request but keep the chain alive.
        schedule(HEALTHY_POLL_MS);
        return;
      }
      inFlight.current = true;
      lastCheck.current = Date.now();
      const s = await fetchApiStatus(); // never throws
      inFlight.current = false;
      if (cancelled) return;
      setStatus(s);
      schedule(nextDelay(s));
    };

    schedule(FIRST_CHECK_MS);
    const onFailure = () => void check(false);
    window.addEventListener(API_REQUEST_FAILED_EVENT, onFailure);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      window.removeEventListener(API_REQUEST_FAILED_EVENT, onFailure);
    };
  }, []);

  if (status.reachable && !status.message) return null;

  return (
    <div
      role="status"
      className={
        status.reachable
          ? "flex items-center gap-2 border-b border-amber-500/30 bg-amber-500/10 px-4 py-2 text-sm text-amber-600"
          : "flex items-center gap-2 border-b border-rose-500/30 bg-rose-500/10 px-4 py-2 text-sm text-rose-500"
      }
    >
      {status.reachable ? <Megaphone className="h-4 w-4 shrink-0" /> : <CloudOff className="h-4 w-4 shrink-0" />}
      <span>
        {status.message ??
          "We're having trouble reaching Foundation right now — check your connection, or hang tight while we look into it."}
      </span>
    </div>
  );
}
