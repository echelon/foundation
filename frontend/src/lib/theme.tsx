import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

type Mode = "light" | "dark";

/**
 * Per-surface theming. Each surface (marketing, dashboard, playground, …) wraps
 * its tree in <ThemeProvider surface="…"> and gets an independent light/dark
 * choice, persisted per surface. The wrapper toggles the `.dark` class locally,
 * so Tailwind `dark:` variants + the CSS token vars flip only within that surface
 * — this mirrors bt's "day/night option for each part of the website".
 */
const ThemeCtx = createContext<{ mode: Mode; toggle: () => void; set: (m: Mode) => void }>({
  mode: "light",
  toggle: () => {},
  set: () => {},
});

function initialMode(surface: string): Mode {
  const stored = localStorage.getItem(`theme:${surface}`) as Mode | null;
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function ThemeProvider({
  surface,
  defaultMode,
  children,
}: {
  surface: string;
  defaultMode?: Mode;
  children: ReactNode;
}) {
  const [mode, setMode] = useState<Mode>(() => defaultMode ?? initialMode(surface));

  useEffect(() => {
    localStorage.setItem(`theme:${surface}`, mode);
  }, [surface, mode]);

  const set = (m: Mode) => setMode(m);
  const toggle = () => setMode((m) => (m === "dark" ? "light" : "dark"));

  return (
    <ThemeCtx.Provider value={{ mode, toggle, set }}>
      <div className={mode === "dark" ? "dark" : ""}>{children}</div>
    </ThemeCtx.Provider>
  );
}

export const useTheme = () => useContext(ThemeCtx);
