import { Link } from "react-router-dom";
import { ArrowLeft, Construction } from "lucide-react";
import { Badge, Button, Card, Container, Logo, ThemeToggle } from "@/components/ui";

/** Temporary placeholder — the internal admin console lands in a later iteration. */
export default function Admin() {
  return (
    <div className="min-h-screen bg-bg text-fg">
      <header className="flex items-center justify-between border-b border-border px-6 py-4">
        <div className="flex items-center gap-3">
          <Logo />
          <Badge tone="amber">Internal</Badge>
        </div>
        <ThemeToggle />
      </header>
      <Container className="grid min-h-[70vh] place-items-center py-20">
        <Card className="max-w-lg p-10 text-center">
          <div className="mx-auto mb-5 grid h-14 w-14 place-items-center rounded-2xl bg-brand/12 text-brand">
            <Construction className="h-7 w-7" />
          </div>
          <h1 className="font-display text-2xl font-bold tracking-tight">Admin console — coming soon</h1>
          <p className="mx-auto mt-3 max-w-sm text-muted">
            The internal admin surface (tenant management, suspensions, refunds via Stripe, model catalog
            controls, and platform-wide observability) is a placeholder for now and will be built in a later
            iteration.
          </p>
          <div className="mt-7 flex justify-center gap-3">
            <Link to="/dashboard">
              <Button variant="outline">
                <ArrowLeft className="h-4 w-4" /> Back to dashboard
              </Button>
            </Link>
            <Link to="/">
              <Button>Home</Button>
            </Link>
          </div>
        </Card>
      </Container>
    </div>
  );
}
