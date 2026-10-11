import { expect, test, type Page } from "@playwright/test";

// 对照 features/employee.feature（Gherkin 中文场景）里的员工主路径：
// 登录入口顺序与无短信、邀请码注册、员工竖屏（无管理侧栏）、绑定食堂、
// 班制只有上二休四/自定义、本周报餐绿（报）/橙（不报）切换。
// 邀请码与食堂由 scripts/e2e.sh 调 e2e/seed.sql 预置，不需要 mock。
const user = `e2e_${Date.now()}`;
const password = "E2e-passw0rd!";
const inviteCode = process.env.DLZ_E2E_INVITE_CODE ?? "DLZ-E2E-INVITE";
const canteen = process.env.DLZ_E2E_CANTEEN ?? "一食堂";

test.describe.configure({ mode: "serial" });

async function login(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /密码登录（兜底）/ }).click();
  await page.getByPlaceholder("账号").fill(user);
  await page.getByPlaceholder("密码").fill(password);
  await page.getByRole("button", { name: "登录", exact: true }).click();
  await expect(page.locator(".emp")).toBeVisible();
}

async function openTab(page: Page, name: string) {
  await page.locator(".van-tabbar-item", { hasText: name }).click();
}

test("未登录只给登录/注册入口，passkey→totp→密码，且无短信 OTP", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("button", { name: "使用 Passkey 登录" })).toBeVisible();
  await expect(page.getByRole("button", { name: "使用 TOTP" })).toBeVisible();
  await expect(page.getByRole("button", { name: /密码登录（兜底）/ })).toBeVisible();
  // 邮箱开关默认关闭，因此不出现 magic link
  await expect(page.getByText("邮箱能力关闭，不显示 magic link")).toBeVisible();
  await expect(page.locator("body")).not.toContainText("短信");
  // 登录页没有侧栏
  await expect(page.locator(".side")).toHaveCount(0);
});

test("邀请码注册后进员工竖屏，默认不报餐且看不到管理端入口", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "有邀请码，去注册" }).click();
  await page.getByPlaceholder("账号").fill(user);
  await page.getByPlaceholder("邀请码").fill(inviteCode);
  await page.getByPlaceholder("密码（至少 8 位）").fill(password);
  await page.getByRole("button", { name: "注册并登录" }).click();

  await expect(page.locator(".emp")).toBeVisible();
  await expect(page.getByText("尚未选择班制，默认不报任何一餐。")).toBeVisible();
  // 员工端没有横屏侧栏，也不显示总管 / 食堂管理能力
  await expect(page.locator(".side")).toHaveCount(0);
  for (const entry of ["人数总览", "菜谱", "邀请码", "审计", "导出", "LLM"]) {
    await expect(page.getByText(entry, { exact: true })).toHaveCount(0);
  }
});

test("绑定食堂后选择班制：只有上二休四与自定义，无上五休二", async ({ page }) => {
  await login(page);

  await openTab(page, "我的");
  await page.getByText("绑定食堂", { exact: true }).click();
  await expect(page.getByText("可以绑定多个食堂。这是报餐时的使用范围。")).toBeVisible();
  await page.getByText(canteen, { exact: true }).click();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("已绑定")).toBeVisible();
  await page.reload();

  await openTab(page, "报餐");
  await page.getByRole("button", { name: "选择班制" }).click();
  await expect(page.getByText("只有上二休四和自定义排班。用描述生成模板是子流程，不是第三种班制。")).toBeVisible();
  await expect(page.getByText("自定义排班", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "用描述生成模板" })).toBeVisible();
  await expect(page.getByText("上五休二")).toHaveCount(0);

  const panel = page.locator("section.panel", { hasText: "上二休四" });
  await panel.locator('input[type="date"]').fill("2025-01-06");
  await panel.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("已设为上二休四")).toBeVisible();
  await expect(page.getByText("尚未选择班制，默认不报任何一餐。")).toHaveCount(0);
});

test("本周报餐按当前食堂生效，餐次可在绿（报）/橙（不报）间切换", async ({ page }) => {
  await login(page);
  await openTab(page, "报餐");
  const legend = page.locator(".legend");
  await expect(legend.getByText("报", { exact: true })).toBeVisible();
  await expect(legend.getByText("不报", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "手动改报餐" }).click();
  await expect(page.getByText("跨食堂按当前选中的食堂生效。锁定时间一到，次日三餐一起锁定。")).toBeVisible();
  // 已绑定食堂会作为报餐范围出现，可以切换
  await expect(page.getByRole("button", { name: canteen, exact: true })).toBeVisible();

  await page.getByRole("button", { name: "下一周" }).click();
  const day = page.locator("article.panel").first();
  await expect(day).toBeVisible();
  const slot = day.getByRole("button", { name: "早", exact: true });
  const before = (await slot.getAttribute("class")) ?? "";
  expect(before).toMatch(/pill/);

  // 绿=报餐（yes），橙=不报（no）。点一次翻转，再点一次翻回。
  const flipped = before.includes("yes") ? /no/ : /yes/;
  const restored = before.includes("yes") ? /yes/ : /no/;
  await slot.click();
  await expect(slot).toHaveClass(flipped);
  await slot.click();
  await expect(slot).toHaveClass(restored);
});
