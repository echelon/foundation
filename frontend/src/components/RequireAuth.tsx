import { Navigate, Outlet, useLocation } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useAuth } from "@/lib/auth";

/**
 * Route guard: children render only for an authenticated user. While the initial
 * `/me` probe is in flight we show a quiet spinner (no redirect flash); with no
 * session we bounce to /login, remembering where they were headed.
 */
export function RequireAuth() {
  const { user, loading } = useAuth();
  const location = useLocation();

  if (loading) {
    return (
      <div className="grid min-h-screen place-items-center bg-bg">
        <Loader2 className="h-6 w-6 animate-spin text-muted" />
      </div>
    );
  }
  if (!user) {
    const redirectTo = location.pathname + location.search + location.hash;
    return <Navigate to={`/login?${new URLSearchParams({ redirect_to: redirectTo })}`} replace />;
  }
  return <Outlet />;
}
