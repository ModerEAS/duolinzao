import { defineConfig, devices } from "@playwright/test";

// 浏览器测试跑在 scripts/e2e.sh 起好的栈上（app 在 http://localhost:8080，Postgres 在 DATABASE_URL）。
// 本地对着 Vite dev server 调试时可设 DLZ_BASE_URL=http://localhost:5173。
export default defineConfig({
  testDir: "./tests",
  outputDir: "./test-results",
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  reporter: [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]],
  use: {
    baseURL: process.env.DLZ_BASE_URL ?? "http://localhost:8080",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
