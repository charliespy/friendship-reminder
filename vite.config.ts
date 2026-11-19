/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri loads the dev server at a fixed port, so it must not move to another one.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Reachable through a .ts.net share of the dev server.
    allowedHosts: [".ts.net"],
    // Cargo's build folders hold tens of thousands of files; watching them floods the watcher.
    watch: { ignored: ["**/target/**", "**/src-tauri/**"] },
    // In browser dev mode (npm run dev:web) the commands go to friends-devserver.
    proxy: { "/__friends": { target: `http://127.0.0.1:${process.env.FRIENDS_DEVSERVER_PORT ?? 1430}`, rewrite: (p) => p.replace(/^\/__friends/, "") } },
  },
  preview: {
    proxy: { "/__friends": { target: `http://127.0.0.1:${process.env.FRIENDS_DEVSERVER_PORT ?? 1430}`, rewrite: (p) => p.replace(/^\/__friends/, "") } },
  },
  build: {
    target: "safari16",
    rollupOptions: {
      input: { dropdown: "index.html", preferences: "preferences.html" },
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    restoreMocks: true,
  },
});
