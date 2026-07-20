import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/lib/theme";
import { AuthProvider } from "@/lib/auth";
import { RequireAuth } from "@/components/RequireAuth";

import MarketingLayout from "@/surfaces/marketing/MarketingLayout";
import Landing from "@/surfaces/marketing/Landing";
import Pricing from "@/surfaces/marketing/Pricing";
import Docs from "@/surfaces/marketing/Docs";
import Catalog from "@/surfaces/models/Catalog";
import ModelPage from "@/surfaces/models/ModelPage";
import ExamplePage from "@/surfaces/models/ExamplePage";
import RankingsOverview from "@/surfaces/rankings/Overview";
import MediaRankings from "@/surfaces/rankings/MediaRankings";
import RankingModel from "@/surfaces/rankings/RankingModel";

import AuthLayout from "@/surfaces/auth/AuthLayout";
import Login from "@/surfaces/auth/Login";
import Signup from "@/surfaces/auth/Signup";
import Reset from "@/surfaces/auth/Reset";
import ResetConfirm from "@/surfaces/auth/ResetConfirm";
import VerifyEmail from "@/surfaces/auth/VerifyEmail";

import DashboardLayout from "@/surfaces/dashboard/DashboardLayout";
import Overview from "@/surfaces/dashboard/Overview";
import Requests from "@/surfaces/dashboard/Requests";
import Billing from "@/surfaces/dashboard/Billing";
import Keys from "@/surfaces/dashboard/Keys";
import Organization from "@/surfaces/dashboard/Organization";
import AuditLogPage from "@/surfaces/dashboard/AuditLog";
import Settings from "@/surfaces/dashboard/Settings";

import Admin from "@/surfaces/admin/Admin";

export default function App() {
  return (
    <AuthProvider>
    <Routes>
      {/* ── Marketing + catalog (public) ── */}
      <Route
        element={
          <ThemeProvider surface="marketing">
            <MarketingLayout />
          </ThemeProvider>
        }
      >
        <Route path="/" element={<Landing />} />
        <Route path="/pricing" element={<Pricing />} />
        <Route path="/docs" element={<Docs />} />
        <Route path="/models" element={<Catalog />} />
        <Route path="/models/:slug" element={<ModelPage />} />
        <Route path="/models/:slug/gallery/:exampleId" element={<ExamplePage />} />
        <Route path="/rankings" element={<RankingsOverview />} />
        <Route path="/rankings/models/:slug" element={<RankingModel />} />
        <Route path="/rankings/:media" element={<MediaRankings />} />
      </Route>

      {/* ── Auth ── */}
      <Route
        element={
          <ThemeProvider surface="auth">
            <AuthLayout />
          </ThemeProvider>
        }
      >
        <Route path="/login" element={<Login />} />
        <Route path="/signup" element={<Signup />} />
        <Route path="/reset" element={<Reset />} />
        {/* The emailed reset link lands here with ?token=… */}
        <Route path="/reset-password" element={<ResetConfirm />} />
        {/* The emailed verification link lands here with ?token=… */}
        <Route path="/verify-email" element={<VerifyEmail />} />
      </Route>

      {/* ── Dashboard (requires login) ── */}
      <Route element={<RequireAuth />}>
        <Route
          element={
            <ThemeProvider surface="dashboard" defaultMode="dark">
              <DashboardLayout />
            </ThemeProvider>
          }
        >
          <Route path="/dashboard" element={<Overview />} />
          <Route path="/dashboard/requests" element={<Requests />} />
          <Route path="/dashboard/billing" element={<Billing />} />
          <Route path="/dashboard/keys" element={<Keys />} />
          <Route path="/dashboard/organization" element={<Organization />} />
          <Route path="/dashboard/audit" element={<AuditLogPage />} />
          <Route path="/dashboard/settings" element={<Settings />} />
        </Route>
      </Route>

      {/* ── Admin (placeholder) ── */}
      <Route
        path="/admin"
        element={
          <ThemeProvider surface="admin" defaultMode="dark">
            <Admin />
          </ThemeProvider>
        }
      />

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
    </AuthProvider>
  );
}
