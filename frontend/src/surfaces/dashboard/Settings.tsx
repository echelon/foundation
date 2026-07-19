import { Button, Card, Input, Label, Select } from "@/components/ui";
import { CURRENT_USER } from "@/lib/mock-api";

export default function Settings() {
  return (
    <div className="max-w-3xl space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Settings</h1>
        <p className="mt-1 text-muted">Manage your profile, organization, and preferences.</p>
      </div>

      {/* ── Profile ── */}
      <Card className="p-6">
        <h2 className="font-display font-semibold">Profile</h2>
        <div className="mt-4 flex items-center gap-4">
          <span className="grid h-16 w-16 place-items-center rounded-full brand-gradient text-xl font-semibold text-white">
            {CURRENT_USER.avatarInitials}
          </span>
          <Button variant="outline" size="sm">
            Change avatar
          </Button>
        </div>
        <div className="mt-5 grid gap-4 sm:grid-cols-2">
          <div>
            <Label>Full name</Label>
            <Input defaultValue={CURRENT_USER.name} />
          </div>
          <div>
            <Label>Email</Label>
            <Input type="email" defaultValue={CURRENT_USER.email} />
          </div>
        </div>
        <div className="mt-5">
          <Button>Save changes</Button>
        </div>
      </Card>

      {/* ── Organization ── */}
      <Card className="p-6">
        <h2 className="font-display font-semibold">Organization</h2>
        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <div>
            <Label>Organization name</Label>
            <Input defaultValue={CURRENT_USER.org} />
          </div>
          <div>
            <Label hint="read-only">Organization ID</Label>
            <Input defaultValue={CURRENT_USER.orgId} readOnly className="font-mono text-[13px] text-muted" />
          </div>
          <div>
            <Label>Your role</Label>
            <Select defaultValue={CURRENT_USER.role}>
              <option value="owner">Owner</option>
              <option value="admin">Admin</option>
              <option value="member">Member</option>
            </Select>
          </div>
          <div>
            <Label>Default model region</Label>
            <Select defaultValue="us">
              <option value="us">United States</option>
              <option value="eu">European Union</option>
              <option value="apac">Asia-Pacific</option>
            </Select>
          </div>
        </div>
      </Card>

      {/* ── Preferences ── */}
      <Card className="p-6">
        <h2 className="font-display font-semibold">Notifications</h2>
        <div className="mt-4 space-y-3">
          {[
            { label: "Low-balance alerts", desc: "Email me before auto-reload triggers." },
            { label: "Weekly usage summary", desc: "A digest of spend and top models." },
            { label: "Product updates", desc: "New models and features." },
          ].map((p, i) => (
            <label key={p.label} className="flex items-center justify-between rounded-lg border border-border bg-surface2/40 px-4 py-3">
              <div>
                <div className="text-sm font-medium text-fg">{p.label}</div>
                <div className="text-xs text-muted">{p.desc}</div>
              </div>
              <input type="checkbox" defaultChecked={i < 2} className="h-4 w-4 accent-[hsl(var(--brand))]" />
            </label>
          ))}
        </div>
      </Card>

      {/* ── Danger zone ── */}
      <Card className="border-rose-500/30 p-6">
        <h2 className="font-display font-semibold text-rose-500">Danger zone</h2>
        <p className="mt-2 text-sm text-muted">Permanently delete this organization and all associated data.</p>
        <Button variant="outline" className="mt-4 border-rose-500/40 text-rose-500 hover:border-rose-500">
          Delete organization
        </Button>
      </Card>
    </div>
  );
}
