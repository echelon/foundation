import type { HistoryFilters } from "@/surfaces/comparison/api";

const historyKeys = [
  "trace_id", "client_ref", "app_ref", "model", "provider", "status", "cost_state", "from", "to",
  "client_unattributed", "app_unattributed", "exact_model",
] as const satisfies readonly (keyof HistoryFilters)[];

export function historySearch(params: URLSearchParams) {
  const filters = new URLSearchParams();
  for (const key of historyKeys) {
    const value = params.get(key);
    const flag = key === "exact_model" || key.endsWith("_unattributed");
    if (value && (!flag || value === "true")) filters.set(key, value);
  }
  return filters.toString();
}

export function setHistoryFilters(params: URLSearchParams, filters: HistoryFilters) {
  for (const key of historyKeys) {
    const value = filters[key];
    if (value) params.set(key, value);
    else params.delete(key);
  }
  return params;
}
