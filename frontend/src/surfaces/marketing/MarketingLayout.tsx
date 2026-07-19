import { Link, NavLink, Outlet } from "react-router-dom";
import { Github, Twitter } from "lucide-react";
import { Button, Container, Logo, ThemeToggle } from "@/components/ui";
import { cn } from "@/lib/cn";

const NAV = [
  { to: "/models", label: "Models" },
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
            <Link to="/login" className="hidden sm:block">
              <Button variant="ghost" size="sm">
                Sign in
              </Button>
            </Link>
            <Link to="/dashboard">
              <Button size="sm">Dashboard</Button>
            </Link>
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
