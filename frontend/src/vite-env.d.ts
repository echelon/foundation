/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Explicit API base URL override (takes precedence over VITE_API_TARGET). */
  readonly VITE_API_BASE_URL?: string;
  readonly VITE_TOKENCRAFT_PACKAGED?: "true";
  /** Which server to hit: "local" (default) or "production". Set at startup (npm run dev:prod). */
  readonly VITE_API_TARGET?: "local" | "production";
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
