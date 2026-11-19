/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Browser dev mode and the e2e tests send the app's commands to friends-devserver through this
// proxy (vite preview uses the same one). Only /invoke goes through; changeOrigin sends the Host the devserver accepts.
const devserverProxy = {
  "/__friends/invoke": {
    target: `http://127.0.0.1:${process.env.FRIENDS_DEVSERVER_PORT ?? 1430}`,
    changeOrigin: true,
    rewrite: (p: string) => p.replace(/^\/__friends/, ""),
  },
};

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
    proxy: devserverProxy,
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
