import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import fs from "fs";
import path from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
const packageJson = JSON.parse(
  fs.readFileSync(new URL("./package.json", import.meta.url), "utf-8")
) as { version: string };

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), react()],
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 24200,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 24201,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }

          const normalizedId = id.replace(/\\/g, "/");
          const matchesPackage = (packageName: string) =>
            normalizedId.includes(`/node_modules/${packageName}/`)
            || normalizedId.includes(
              `/node_modules/.pnpm/${packageName.replace("/", "+")}@`
            );

          if (
            matchesPackage("react")
            || matchesPackage("react-dom")
            || matchesPackage("scheduler")
            || matchesPackage("react-router-dom")
            || matchesPackage("@remix-run/router")
          ) {
            return "react-vendor";
          }

          if (
            matchesPackage("i18next")
            || matchesPackage("react-i18next")
            || matchesPackage("i18next-browser-languagedetector")
          ) {
            return "i18n-vendor";
          }

          if (matchesPackage("lucide-react") || normalizedId.includes("/node_modules/@lobehub/")) {
            return "icon-vendor";
          }

          if (normalizedId.includes("/node_modules/@tauri-apps/")) {
            return "tauri-vendor";
          }

          return "ui-vendor";
        },
      },
    },
  },

  // Vitest configuration
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
  },
}));
