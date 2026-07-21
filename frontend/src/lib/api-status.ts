import { API_BASE_URL } from "./api";

/**
 * The "is it just me?" probe (`GET /v1/api_status`) — a little helper, deliberately NOT part
 * of the product's critical path. `fetchApiStatus` never throws and never blocks rendering;
 * callers poll it occasionally (see ApiStatusBanner) or after a real request fails.
 */
export type ApiStatus = {
  /** The API answered at all. */
  reachable: boolean;
  /** Operator-set status/downtime message (API_STATUS_MESSAGE on the backend). */
  message: string | null;
};

/** Fired by the api client when a request dies on network error or a 5xx — a nudge to
 * re-check status sooner than the next scheduled poll. */
export const API_REQUEST_FAILED_EVENT = "foundation:api-request-failed";

export async function fetchApiStatus(): Promise<ApiStatus> {
  try {
    const resp = await fetch(`${API_BASE_URL}/v1/api_status`, { signal: AbortSignal.timeout(5000) });
    if (!resp.ok) return { reachable: false, message: null };
    const json = (await resp.json()) as { ok?: boolean; message?: string | null } | null;
    return { reachable: true, message: json?.message ?? null };
  } catch {
    return { reachable: false, message: null };
  }
}
