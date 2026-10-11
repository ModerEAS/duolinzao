import { expect, type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";

// features/employee.feature 中员工主路径的薄封装。
// 选择器与流程对照 web/src/pages/EmployeePage.vue、LoginPage.vue；
// 邀请码与食堂由 scripts/e2e.sh 调 e2e/seed.sql 预置，不需要 mock。
export const password = "E2e-passw0rd!";
export const inviteCode = process.env.DLZ_E2E_INVITE_CODE ?? "DLZ-E2E-INVITE";
export const canteen = process.env.DLZ_E2E_CANTEEN ?? "一食堂";

let seq = 0;
let currentUser = "";

// 场景跑在各自进程里（模块状态不共享），本轮注册的账号写文件共享。
// DB 跨轮持久化：旧文件指向的账号若还能登录就复用。
const userFile = process.env.DLZ_E2E_USER_FILE ?? ".tmp/e2e-user";

function readSharedUser(): string {
  try {
    return readFileSync(userFile, "utf8").trim();
  } catch {
    return "";
  }
}

export async function openTab(page: Page, name: string) {
  await page.locator(".van-tabbar-item", { hasText: name }).click();
}

// 注册一个全新账号并登录（用户名保证唯一，套件可重复跑）。
export async function registerNewUser(page: Page) {
  currentUser = `e2e_${Date.now()}_${seq++}`;
  await page.goto("/");
  await page.getByRole("button", { name: "有邀请码，去注册" }).click();
  await page.getByPlaceholder("账号").fill(currentUser);
  await page.getByPlaceholder("邀请码").fill(inviteCode);
  await page.getByPlaceholder("密码（至少 8 位）").fill(password);
  await page.getByRole("button", { name: "注册并登录" }).click();
  await expect(page.locator(".emp")).toBeVisible();
  mkdirSync(".tmp", { recursive: true });
  writeFileSync(userFile, currentUser);
}

// 保证已登录：优先用本轮已注册账号（文件共享，跨进程），
// 登录不上（单独跑某场景、旧账号不可用）则现场注册。套件里只有注册场景消费邀请码。
export async function ensureLoggedIn(page: Page) {
  const saved = currentUser || readSharedUser();
  if (saved) {
    currentUser = saved;
    await page.goto("/");
    await page.getByRole("button", { name: /密码登录（兜底）/ }).click();
    await page.getByPlaceholder("账号").fill(currentUser);
    await page.getByPlaceholder("密码").fill(password);
    await page.getByRole("button", { name: "登录", exact: true }).click();
    try {
      await expect(page.locator(".emp")).toBeVisible({ timeout: 8000 });
      return;
    } catch {
      // 登录不上，继续往下现场注册。
    }
  }
  await registerNewUser(page);
}

// 保证已绑定至少一个食堂（已绑定则跳过，避免重复点导致反选解绑）。
export async function ensureBound(page: Page) {
  await ensureLoggedIn(page);
  await openTab(page, "报餐");
  await page.getByRole("button", { name: "手动改报餐" }).click();
  try {
    await expect(page.getByRole("button", { name: canteen, exact: true }).first()).toBeVisible({
      timeout: 4000,
    });
    return;
  } catch {
    // 还没绑定，走绑定流程。
  }
  await openTab(page, "我的");
  await page.getByText("绑定食堂", { exact: true }).click();
  await page.getByText(canteen, { exact: true }).click();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("已绑定")).toBeVisible();
}

// 选择上二休四并保存（重复保存同一预设是幂等的）。
export async function chooseShift(page: Page) {
  await ensureBound(page);
  await openTab(page, "报餐");
  await page.getByRole("button", { name: "选择班制" }).click();
  const panel = page.locator("section.panel", { hasText: "上二休四" });
  await panel.locator('input[type="date"]').fill("2025-01-06");
  await panel.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("已设为上二休四")).toBeVisible();
}
