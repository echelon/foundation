import { useState } from "react";
import { Link } from "react-router-dom";
import { CheckCircle2 } from "lucide-react";
import { Button, Input, Label } from "@/components/ui";
import { AuthCard } from "./AuthCard";

export default function Reset() {
  const [sent, setSent] = useState(false);
  return (
    <AuthCard
      title="Reset password"
      subtitle={sent ? "Check your inbox for the reset link." : "We'll email you a link to reset your password."}
      footer={
        <Link to="/login" className="font-medium text-brand hover:underline">
          Back to sign in
        </Link>
      }
    >
      {sent ? (
        <div className="flex items-start gap-3 rounded-lg bg-emerald-500/10 p-4 text-sm text-emerald-500">
          <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0" />
          <p>If an account exists for that email, a reset link is on its way.</p>
        </div>
      ) : (
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            setSent(true);
          }}
        >
          <div>
            <Label>Email</Label>
            <Input type="email" placeholder="you@company.com" required />
          </div>
          <Button type="submit" className="w-full">
            Send reset link
          </Button>
        </form>
      )}
    </AuthCard>
  );
}
