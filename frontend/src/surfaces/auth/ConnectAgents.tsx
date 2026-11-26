import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";

import { API_BASE_URL } from "@/lib/api";
import { safeRedirectPath } from "@/lib/redirect";

type Discovery = { resource: string };

function useConnector(enabled: boolean) {
  const [discovery, setDiscovery] = useState<Discovery | "unavailable" | null>(null);

  useEffect(() => {
    if (!enabled) return;
    const request = new AbortController();
    fetch(`${API_BASE_URL}/.well-known/oauth-protected-resource/mcp`, {
      credentials: "omit",
      signal: request.signal,
    })
      .then(async (response) => {
        if (!response.ok) return "unavailable" as const;
        const metadata: unknown = await response.json();
        if (typeof metadata !== "object" || !metadata || !("resource" in metadata)
          || typeof metadata.resource !== "string") return "unavailable" as const;
        const resource = new URL(metadata.resource);
        if (resource.pathname !== "/mcp" || resource.search || resource.hash || resource.username || resource.password
          || (resource.protocol !== "https:" && resource.protocol !== "http:")) return "unavailable" as const;
        return { resource: resource.href };
      })
      .then((metadata) => {
        if (!request.signal.aborted) setDiscovery(metadata);
      })
      .catch(() => {
        if (!request.signal.aborted) setDiscovery("unavailable");
      });
    return () => request.abort();
  }, [enabled]);

  return discovery;
}

export default function ConnectAgents() {
  const [params] = useSearchParams();
  const raw = params.get("continue");
  const path = safeRedirectPath(raw);
  const continuation = path?.startsWith("/mcp/oauth/authorize?") ? path
    : path === "/mcp/oauth/connections" ? path : null;
  const discovery = useConnector(raw === null);

  return (
    <section className="space-y-4">
      <h1 className="font-display text-2xl font-bold">Connect an agent</h1>
      {raw !== null ? (
        continuation ? (
          <>
            <p>Continue to TokenCraft to review the requested access.</p>
            <a className="inline-block font-semibold text-brand underline" href={`${API_BASE_URL}${continuation}`}>Review access</a>
          </>
        ) : <p role="alert">This authorization link is invalid.</p>
      ) : discovery === null ? (
        <p role="status">Checking connector availability…</p>
      ) : discovery === "unavailable" ? (
        <p>This TokenCraft deployment has not enabled MCP. Use the <Link className="underline" to="/docs#agent-setup">coding agent quickstart</Link> to connect over HTTP.</p>
      ) : (
        <>
          <p>Add this server URL as a custom connector in ChatGPT or Claude. Sign in with your TokenCraft account and choose a project.</p>
          <label className="block text-sm font-semibold" htmlFor="mcp-url">MCP server URL</label>
          <input id="mcp-url" className="w-full rounded border border-line bg-surface2 p-2 font-mono text-sm" value={discovery.resource} readOnly />
          <a className="inline-block underline" href={`${API_BASE_URL}/mcp/oauth/connections`}>Manage agent connections</a>
        </>
      )}
    </section>
  );
}
