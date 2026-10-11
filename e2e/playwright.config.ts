import { defineConfig, devices } from "@playwright/test";
import { defineBddConfig } from "playwright-bdd";

// Gherkin 特性文件住在仓库根 features/（中文，# language: zh-CN），
// bddgen 把非 @wip 场景生成到 .features-gen/，再由 playwright test 执行。
// @wip 场景（LLM、评分、设备/Passkey、食堂/管理/总管横屏等）默认不生成、不跑，见 e2e/README.md。
const testDir = defineBddConfig({
  featuresRoot: "..",
  features: "../features/*.feature",
  steps: "steps/*.ts",
  tags: "not @wip",
});

// 浏览器测试跑在 scripts/e2e.sh 起好的栈上（app 在 http://localhost:8080，Postgres 在 DATABASE_URL）。
// 本地对着 Vite dev server 调试时可设 DLZ_BASE_URL=http://localhost:5173。
export default defineConfig({
  testDir,
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
