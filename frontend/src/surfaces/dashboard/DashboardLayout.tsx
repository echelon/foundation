import { Link, NavLink, Outlet, useNavigate } from "react-router-dom";
import {
  CreditCard,
  KeyRound,
  LayoutDashboard,
  ListTree,
  LogOut,
  Settings as SettingsIcon,
  Sparkles,
  Wallet,
} from "lucide-react";
import { Badge, Logo, ThemeToggle } from "@/components/ui";
import { UserAvatar } from "@/components/UserAvatar";
import { BALANCE } from "@/lib/mock-api";
import { useAuth } from "@/lib/auth";
import { cn } from "@/lib/cn";

const NAV = [
  { to: "/dashboard", label: "Overview", icon: LayoutDashboard, end: true },
  { to: "/dashboard/requests", label: "Requests", icon: ListTree },
  { to: "/dashboard/keys", label: "API keys", icon: KeyRound },
  { to: "/dashboard/billing", label: "Billing", icon: CreditCard },
  { to: "/dashboard/settings", label: "Settings", icon: SettingsIcon },
];

export default function DashboardLayout() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  // RequireAuth guarantees a user before this renders; the guard is just for type-safety.
  if (!user) return null;

  const handleLogout = async () => {
    await logout();
    navigate("/", { replace: true });
  };

  return (
    <div className="min-h-screen bg-bg text-fg">
      <div className="flex">
        {/* ── Sidebar ── */}
        <aside className="sticky top-0 hidden h-screen w-64 shrink-0 flex-col border-r border-border bg-surface/40 lg:flex">
          <div className="px-5 py-5">
            <Logo />
          </div>
          <nav className="flex-1 space-y-1 px-3">
            {NAV.map((n) => (
              <NavLink
                key={n.to}
                to={n.to}
                end={n.end}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition",
                    isActive ? "bg-brand/12 text-brand" : "text-muted hover:bg-surface2 hover:text-fg",
                  )
                }
              >
                <n.icon className="h-4 w-4" />
                {n.label}
              </NavLink>
            ))}
            <Link
              to="/models"
              className="mt-2 flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-muted transition hover:bg-surface2 hover:text-fg"
            >
              <Sparkles className="h-4 w-4" />
              Playground
            </Link>
          </nav>

          {/* Balance card */}
          <div className="m-3 rounded-xl border border-border bg-surface p-4">
            <div className="flex items-center gap-2 text-xs text-muted">
              <Wallet className="h-3.5 w-3.5" /> Credits
            </div>
            <div className="mt-1 font-display text-xl font-bold">${BALANCE.credits.toFixed(2)}</div>
            <Link to="/dashboard/billing" className="mt-2 block text-xs font-medium text-brand hover:underline">
              Add credits →
            </Link>
          </div>
        </aside>

        {/* ── Main ── */}
        <div className="flex min-w-0 flex-1 flex-col">
          <header className="sticky top-0 z-30 flex h-16 items-center justify-between border-b border-border bg-bg/80 px-5 backdrop-blur sm:px-8">
            <div className="flex items-center gap-3">
              <span className="font-display font-semibold">{user.display_name ?? user.email}</span>
              {!user.email_verified && <Badge tone="amber">Email unverified</Badge>}
            </div>
            <div className="flex items-center gap-3">
              <ThemeToggle />
              <Link to="/dashboard/settings" className="flex items-center gap-2">
                <span className="hidden text-right sm:block">
                  <span className="block text-sm font-medium leading-tight">{user.display_name ?? "—"}</span>
                  <span className="block text-xs text-muted">{user.email}</span>
                </span>
                <UserAvatar user={user} className="h-9 w-9 text-sm" />
              </Link>
              <button
                onClick={handleLogout}
                title="Sign out"
                className="rounded-lg p-2 text-muted transition hover:bg-surface2 hover:text-fg"
              >
                <LogOut className="h-4 w-4" />
              </button>
            </div>
          </header>
          <main className="min-w-0 flex-1 px-5 py-8 sm:px-8">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}
