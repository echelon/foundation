import { useState } from "react";
import { Copy, Eye, EyeOff, Plus, Trash2 } from "lucide-react";
import { Badge, Button, Card } from "@/components/ui";
import { API_KEYS } from "@/lib/mock-api";

export default function Keys() {
  const [revealed, setRevealed] = useState<string | null>(null);

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl font-bold tracking-tight">API keys</h1>
          <p className="mt-1 text-muted">One secret authenticates every endpoint. Rotate anytime.</p>
        </div>
        <Button>
          <Plus className="h-4 w-4" /> Create key
        </Button>
      </div>

      <Card className="flex items-start gap-3 border-amber-500/30 bg-amber-500/[0.06] p-4">
        <p className="text-sm text-muted">
          Treat keys like passwords. Use <code className="rounded bg-surface2 px-1.5 py-0.5">sk_live_</code> keys
          in production and <code className="rounded bg-surface2 px-1.5 py-0.5">sk_test_</code> keys for sandbox
          traffic. A key is shown in full only once, at creation.
        </p>
      </Card>

      <Card className="overflow-hidden">
        <div className="scrollbar-thin overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="border-b border-border text-left text-muted">
              <tr>
                <th className="px-5 py-3 font-medium">Name</th>
                <th className="px-5 py-3 font-medium">Key</th>
                <th className="px-5 py-3 font-medium">Env</th>
                <th className="px-5 py-3 font-medium">Created</th>
                <th className="px-5 py-3 font-medium">Last used</th>
                <th className="px-5 py-3"></th>
              </tr>
            </thead>
            <tbody>
              {API_KEYS.map((k) => {
                const isRevealed = revealed === k.id;
                return (
                  <tr key={k.id} className="border-b border-border/60 last:border-0">
                    <td className="px-5 py-3 font-medium text-fg">{k.name}</td>
                    <td className="px-5 py-3">
                      <div className="flex items-center gap-2">
                        <code className="font-mono text-[13px] text-muted">
                          {k.prefix}
                          {isRevealed ? `${"x".repeat(16)}${k.last4}` : `••••••••${k.last4}`}
                        </code>
                        <button onClick={() => setRevealed(isRevealed ? null : k.id)} className="text-muted hover:text-fg">
                          {isRevealed ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                        </button>
                        <button className="text-muted hover:text-fg">
                          <Copy className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </td>
                    <td className="px-5 py-3">
                      <Badge tone={k.env === "live" ? "brand" : "muted"}>{k.env}</Badge>
                    </td>
                    <td className="px-5 py-3 text-muted">{k.created}</td>
                    <td className="px-5 py-3 text-muted">{k.lastUsed ?? "never"}</td>
                    <td className="px-5 py-3 text-right">
                      <button className="text-muted hover:text-rose-500">
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
