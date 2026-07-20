import { Navigate, Outlet, useLocation, useSearchParams } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useAuth } from "@/lib/auth";
import { safeRedirectPath } from "@/lib/redirect";

/**
 * The inverse of [`RequireAuth`]: sessionless pages (login, signup, reset request) render
 * only for signed-out visitors. A logged-in user is bounced to `?redirect_to={path}` when
 * present and same-origin-safe ([`safeRedirectPath`]), else back to where RequireAuth
 * bounced them from, else the dashboard. (Same precedence as the login form's own
 * post-submit navigation, so it doesn't matter which fires first.)
 */
export function GuestOnly() {
  const { user, loading } = useAuth();
  const location = useLocation();
  const [params] = useSearchParams();

  if (loading) {
    return (
      <div className="grid min-h-screen place-items-center bg-bg">
        <Loader2 className="h-6 w-6 animate-spin text-muted" />
      </div>
    );
  }
  if (user) {
    const from = (location.state as { from?: string } | null)?.from;
    return <Navigate to={safeRedirectPath(params.get("redirect_to")) ?? from ?? "/dashboard"} replace />;
  }
  return <Outlet />;
}
