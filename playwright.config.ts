import { defineConfig, devices } from "@playwright/test";
import { SERVER_PORT as serverPort, WEB_PORT as webPort } from "./e2e/ports";

/**
 * End-to-end tests: the built UI in WebKit (the engine of the macOS app's web view), talking to
 * friends-devserver, which runs the real rules over a scratch data file with a fixed "today".
 * Ports: e2e/ports.ts (FRIENDS_E2E_SERVER_PORT / FRIENDS_E2E_WEB_PORT, defaults 1440 / 1441).
 */

export default defineConfig({
  testDir: "e2e/tests",
  fullyParallel: false,
  // One data file behind one server: tests run one at a time.
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: `http://127.0.0.1:${webPort}`,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "webkit", use: { ...devices["Desktop Safari"], viewport: { width: 1100, height: 760 } } }],
  webServer: [
    {
      command: `cargo run -q -p friends-devserver -- --port ${serverPort} --data target/e2e/friends.json --today 2026-11-18`,
      url: `http://127.0.0.1:${serverPort}/__test/health`,
      reuseExistingServer: false,
      timeout: 300_000,
    },
    {
      command: `npx vite build --outDir target/e2e/dist --emptyOutDir && npx vite preview --outDir target/e2e/dist --host 127.0.0.1 --port ${webPort} --strictPort`,
      env: { VITE_BROWSER_BRIDGE: "1", FRIENDS_DEVSERVER_PORT: String(serverPort) },
      url: `http://127.0.0.1:${webPort}/`,
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
