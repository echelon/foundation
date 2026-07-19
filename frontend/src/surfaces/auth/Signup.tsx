import { Link, useNavigate } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { AuthCard, Divider, SsoRow } from "./AuthCard";

export default function Signup() {
  const navigate = useNavigate();
  return (
    <AuthCard
      title="Create your account"
      subtitle="Start generating in minutes. No card required to explore."
      footer={
        <>
          Already have an account?{" "}
          <Link to="/login" className="font-medium text-brand hover:underline">
            Sign in
          </Link>
        </>
      }
    >
      <SsoRow />
      <Divider />
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          navigate("/dashboard");
        }}
      >
        <div>
          <Label>Full name</Label>
          <Input placeholder="Ada Rivera" required />
        </div>
        <div>
          <Label>Work email</Label>
          <Input type="email" placeholder="you@company.com" required />
        </div>
        <div>
          <Label>Password</Label>
          <Input type="password" placeholder="At least 12 characters" required />
        </div>
        <Button type="submit" className="w-full">
          Create account
        </Button>
        <p className="text-center text-xs text-muted">
          By continuing you agree to the{" "}
          <a href="#" className="text-muted underline hover:text-fg">
            Terms
          </a>{" "}
          and{" "}
          <a href="#" className="text-muted underline hover:text-fg">
            Privacy Policy
          </a>
          .
        </p>
      </form>
    </AuthCard>
  );
}
