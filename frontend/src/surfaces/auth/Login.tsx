import { Link, useNavigate } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { AuthCard, Divider, SsoRow } from "./AuthCard";

export default function Login() {
  const navigate = useNavigate();
  return (
    <AuthCard
      title="Welcome back"
      subtitle="Sign in to your Foundation account."
      footer={
        <>
          New here?{" "}
          <Link to="/signup" className="font-medium text-brand hover:underline">
            Create an account
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
          <Label>Email</Label>
          <Input type="email" placeholder="you@company.com" defaultValue="ada@lumenlabs.io" required />
        </div>
        <div>
          <Label hint="">Password</Label>
          <Input type="password" placeholder="••••••••" defaultValue="password" required />
          <div className="mt-1.5 text-right">
            <Link to="/reset" className="text-xs text-muted hover:text-fg">
              Forgot password?
            </Link>
          </div>
        </div>
        <Button type="submit" className="w-full">
          Sign in
        </Button>
      </form>
    </AuthCard>
  );
}
