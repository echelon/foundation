import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import path from "node:path";

export default defineConfig({
  plugins: [
    react(),
    {
      name: "release-openapi",
      apply: "build",
      buildStart() {
        const contract = path.resolve(__dirname, "../contracts/openapi.json");
        this.addWatchFile(contract);
        this.emitFile({ type: "asset", fileName: "docs/openapi.json", source: readFileSync(contract) });
      },
    },
  ],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  server: { port: 3000 },
});
