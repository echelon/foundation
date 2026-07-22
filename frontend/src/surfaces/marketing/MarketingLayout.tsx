import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Outlet, useNavigate } from "react-router-dom";
import { Github, LayoutDashboard, LogOut, Settings, Twitter } from "lucide-react";
import { Button, Container, Logo, ThemeToggle } from "@/components/ui";
import { UserAvatar } from "@/components/UserAvatar";
import { useAuth } from "@/lib/auth";
import { cn } from "@/lib/cn";

const NAV = [
  { to: "/models", label: "Models" },
  { to: "/rankings", label: "Rankings" },
  { to: "/pricing", label: "Pricing" },
  { to: "/docs", label: "Docs" },
];

export default function MarketingLayout() {
  return (
    <div className="min-h-screen bg-bg text-fg">
      <header className="sticky top-0 z-40 border-b border-border/70 bg-bg/80 backdrop-blur">
        <Container className="flex h-16 items-center justify-between gap-4">
          <div className="flex items-center gap-8">
            <Logo />
            <nav className="hidden items-center gap-1 md:flex">
              {NAV.map((n) => (
                <NavLink
                  key={n.to}
                  to={n.to}
                  className={({ isActive }) =>
                    cn(
                      "rounded-lg px-3 py-1.5 text-sm font-medium transition",
                      isActive ? "bg-surface2 text-fg" : "text-muted hover:text-fg",
                    )
                  }
                >
                  {n.label}
                </NavLink>
              ))}
            </nav>
          </div>
          <div className="flex items-center gap-2">
            <ThemeToggle />
            <HeaderAuth />
          </div>
        </Container>
      </header>

      <main>
        <Outlet />
      </main>

      <footer className="border-t border-border/70 bg-surface/40">
        <Container className="grid gap-8 py-12 sm:grid-cols-2 lg:grid-cols-4">
          <div className="space-y-3">
            <Logo />
            <p className="max-w-xs text-sm text-muted">
              One accessible API for the best image, video, 3D, and language models.
            </p>
            <div className="flex gap-2 pt-1 text-muted">
              <a href="#" className="grid h-8 w-8 place-items-center rounded-lg hover:bg-surface2 hover:text-fg">
                <Github className="h-4 w-4" />
              </a>
              <a href="#" className="grid h-8 w-8 place-items-center rounded-lg hover:bg-surface2 hover:text-fg">
                <Twitter className="h-4 w-4" />
              </a>
            </div>
          </div>
          <FooterCol title="Product" links={["Models", "Pricing", "Playground", "Changelog"]} />
          <FooterCol title="Developers" links={["Docs", "API reference", "Status", "Rate limits"]} />
          <FooterCol title="Company" links={["About", "Careers", "Terms", "Privacy"]} />
        </Container>
        <Container className="flex flex-col items-center justify-between gap-2 border-t border-border/60 py-6 text-xs text-muted sm:flex-row">
          <span>© 2026 Foundation, Inc. All rights reserved.</span>
          <span>Mock preview · frontend only</span>
        </Container>
      </footer>
    </div>
  );
}

/**
 * The header's auth corner. Guests get Sign in + Dashboard; a signed-in user gets their
 * avatar with a context menu (info, Dashboard, User Settings, Sign out). Renders nothing
 * during the initial /me probe so "Sign in" never flashes for someone who has a session.
 */
function HeaderAuth() {
  const { user, loading } = useAuth();
  if (loading) return <div className="h-8 w-8" aria-hidden />;
  if (!user) {
    return (
      <>
        <Link to="/login" className="hidden sm:block">
          <Button variant="ghost" size="sm">
            Sign in
          </Button>
        </Link>
        <Link to="/dashboard">
          <Button size="sm">Dashboard</Button>
        </Link>
      </>
    );
  }
  return <UserMenu />;
}

function UserMenu() {
  const { user, logout } = useAuth();
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  if (!user) return null;

  const handleLogout = async () => {
    setOpen(false);
    await logout();
    navigate("/", { replace: true });
  };

  const itemClass =
    "flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted transition hover:bg-surface2 hover:text-fg";

  return (
    <div ref={rootRef} className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="menu"
        aria-expanded={open}
        className="rounded-full transition hover:opacity-85 focus:outline-none focus:ring-2 focus:ring-brand/50"
      >
        <UserAvatar user={user} className="h-9 w-9 text-sm" />
      </button>
      {open && (
        <div
          role="menu"
          className="absolute right-0 z-50 mt-2 w-64 rounded-xl border border-border bg-surface p-1.5 shadow-xl"
        >
          <div className="px-3 py-2">
            <div className="truncate text-sm font-medium text-fg">{user.display_name ?? "—"}</div>
            <div className="truncate text-xs text-muted">{user.email}</div>
          </div>
          <div className="my-1 border-t border-border/60" />
          <Link to="/dashboard" role="menuitem" className={itemClass} onClick={() => setOpen(false)}>
            <LayoutDashboard className="h-4 w-4" />
            Dashboard
          </Link>
          <Link
            to="/dashboard/user/settings"
            role="menuitem"
            className={itemClass}
            onClick={() => setOpen(false)}
          >
            <Settings className="h-4 w-4" />
            User Settings
          </Link>
          <div className="my-1 border-t border-border/60" />
          <button role="menuitem" onClick={handleLogout} className={itemClass}>
            <LogOut className="h-4 w-4" />
            Sign out
          </button>
        </div>
      )}
    </div>
  );
}

function FooterCol({ title, links }: { title: string; links: string[] }) {
  return (
    <div>
      <h4 className="mb-3 text-sm font-semibold text-fg">{title}</h4>
      <ul className="space-y-2 text-sm text-muted">
        {links.map((l) => (
          <li key={l}>
            <a href="#" className="hover:text-fg">
              {l}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
