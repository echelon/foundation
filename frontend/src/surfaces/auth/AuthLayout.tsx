import { Outlet } from "react-router-dom";
import { Logo, ThemeToggle } from "@/components/ui";

export default function AuthLayout() {
  return (
    <div className="relative min-h-screen bg-bg text-fg">
      <div className="grid-bg pointer-events-none absolute inset-0 opacity-50" />
      <div className="pointer-events-none absolute -top-32 left-1/2 h-80 w-[36rem] -translate-x-1/2 rounded-full brand-gradient opacity-20 blur-3xl" />
      <header className="relative flex items-center justify-between px-6 py-5">
        <Logo />
        <ThemeToggle />
      </header>
      <main className="relative grid place-items-center px-5 py-12">
        <Outlet />
      </main>
    </div>
  );
}
