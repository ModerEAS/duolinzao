import { defineConfig, devices } from "@playwright/test";
import { defineBddConfig } from "playwright-bdd";

// Gherkin 特性文件住在仓库根 features/（中文，# language: zh-CN），
// bddgen 把非 @wip 非 @scale 场景生成到 .features-gen/，再由 playwright test 执行。
// @wip 场景默认不生成、不跑；@scale 旅程体量大，只在 DLZ_SCALE 档位下单独跑，见 e2e/README.md。
const testDir = defineBddConfig({
  featuresRoot: "..",
  features: "../features/*.feature",
  steps: "steps/*.ts",
  tags: process.env.DLZ_BDD_TAGS ?? "not @wip and not @scale",
});

// 浏览器测试跑在 scripts/e2e.sh 起好的栈上（app 在 http://localhost:8080，Postgres 在 DATABASE_URL）。
// 本地对着 Vite dev server 调试时可设 DLZ_BASE_URL=http://localhost:5173。
export default defineConfig({
  testDir,
  outputDir: "./test-results",
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  // @scale 单场景做几十到上百次 UI 操作：档位运行时放宽到 30 分钟；默认（切片场景）仍 60 秒。
  timeout: Number(process.env.DLZ_E2E_TIMEOUT_MS ?? 60_000),
  expect: { timeout: 10_000 },
  reporter: [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]],
  use: {
    baseURL: process.env.DLZ_BASE_URL ?? "http://localhost:8080",
    // 单次点击/填写上限：scale 长链路里无限等待等于卡死，30 秒超时转成可定位的失败。
    actionTimeout: 30_000,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
