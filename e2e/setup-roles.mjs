// e2e 角色预置：二食堂兜底 + 本地脚本 LLM 渠道 + canteen/admin 账号与角色分配。
// 全部走真实 API（注册 / 登录 / 建食堂 / 角色分配），幂等，可反复跑。
// 由 scripts/e2e.sh 在 seed.sql 之后调用；不写密钥文件，只用环境变量。
const BASE = process.env.DLZ_BASE_URL ?? "http://localhost:8080";
const ROOT_USER = process.env.DLZ_SUPER_USER ?? "root";
const ROOT_PASS = process.env.DLZ_SUPER_PASS ?? "root-password-1";
const CANTEEN1 = process.env.DLZ_E2E_CANTEEN ?? "一食堂";
const CANTEEN2 = process.env.DLZ_E2E_CANTEEN2 ?? "二食堂";
const CANTEEN_USER = process.env.DLZ_E2E_CANTEEN_USER ?? "e2e_canteen";
const CANTEEN_PASS = process.env.DLZ_E2E_CANTEEN_PASS ?? "E2e-canteen-pass1!";
const ADMIN_USER = process.env.DLZ_E2E_ADMIN_USER ?? "e2e_admin";
const ADMIN_PASS = process.env.DLZ_E2E_ADMIN_PASS ?? "E2e-admin-pass1!";

async function req(path, { method = "GET", body, cookie } = {}) {
  const headers = { "x-dlz-requested": "1" };
  if (body !== undefined) headers["content-type"] = "application/json";
  if (cookie) headers.cookie = cookie;
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = null;
  }
  const setCookie = res.headers.get("set-cookie") ?? "";
  const session = (setCookie.match(/dlz_session=([^;]+)/) ?? [])[1];
  return { status: res.status, json, session: session ? `dlz_session=${session}` : null };
}

async function login(username, password) {
  const r = await req("/api/session/password", {
    method: "POST",
    body: { username, password },
  });
  if (r.status !== 200 || !r.session) return null;
  return r.session;
}

async function main() {
  // 健康检查等调用方保证；这里只做幂等预置。
  const root = await login(ROOT_USER, ROOT_PASS);
  if (!root) throw new Error("root 登录失败，检查超管密码与服务");
  console.log("setup: root ok");

  // 食堂：seed.sql 已插两家，这里只读 id，缺了才补建。
  let canteens = (await req("/api/canteens", { cookie: root })).json?.items ?? [];
  for (const name of [CANTEEN1, CANTEEN2]) {
    if (!canteens.some((c) => c.name === name)) {
      const created = await req("/api/admin/canteens", {
        method: "POST",
        body: { name },
        cookie: root,
      });
      if (created.status !== 201) throw new Error(`建食堂 ${name} 失败: ${created.status}`);
      console.log(`setup: canteen created ${name}`);
      canteens = (await req("/api/canteens", { cookie: root })).json?.items ?? [];
    }
  }
  const idOf = (name) => canteens.find((c) => c.name === name)?.id;
  const c1 = idOf(CANTEEN1);
  const c2 = idOf(CANTEEN2);
  if (!c1 || !c2) throw new Error("食堂 id 缺失");

  // LLM：至少一条可用渠道。e2e 员工生成场景用本地脚本（无外部调用）。
  const channels = (await req("/api/admin/llm-channels", { cookie: root })).json?.channels ?? [];
  if (!channels.some((c) => c.enabled)) {
    const put = await req("/api/admin/llm-channels", {
      method: "PUT",
      body: { channels: [{ kind: "local_script", enabled: true }] },
      cookie: root,
    });
    if (put.status !== 200) throw new Error(`配 LLM 本地脚本失败: ${put.status}`);
    console.log("setup: llm local_script enabled");
  } else {
    console.log(`setup: llm channels exist (${channels.length})`);
  }

  // 账号：能登录就复用；不能则发邀请码注册。
  async function ensureUser(username, password) {
    if (await login(username, password)) {
      console.log(`setup: user exists ${username}`);
      return;
    }
    const inv = await req("/api/admin/invites", { method: "POST", cookie: root });
    if (inv.status !== 201 || !inv.json?.code) throw new Error(`发邀请码失败: ${inv.status}`);
    const reg = await req("/api/register", {
      method: "POST",
      body: { username, invite_code: inv.json.code, password },
    });
    if (reg.status !== 201) throw new Error(`注册 ${username} 失败: ${reg.status} ${JSON.stringify(reg.json)}`);
    const check = await login(username, password);
    if (!check) throw new Error(`新账号 ${username} 登录失败`);
    console.log(`setup: user registered ${username}`);
  }
  await ensureUser(CANTEEN_USER, CANTEEN_PASS);
  await ensureUser(ADMIN_USER, ADMIN_PASS);

  // 角色分配：canteen=[一食堂]；admin=[一食堂]（二食堂留在范围外，供越权断言）。
  const accounts = (await req("/api/admin/accounts", { cookie: root })).json?.items ?? [];
  const aidOf = (name) => accounts.find((a) => a.username === name)?.id;
  async function assign(username, role, canteen_ids) {
    const account_id = aidOf(username);
    if (!account_id) throw new Error(`找不到账号 ${username}`);
    const r = await req("/api/admin/role-assignments", {
      method: "POST",
      body: { account_id, role, canteen_ids },
      cookie: root,
    });
    if (r.status !== 200) throw new Error(`角色分配 ${username}/${role} 失败: ${r.status} ${JSON.stringify(r.json)}`);
    console.log(`setup: role ${username} -> ${role} [${canteen_ids.length}]`);
  }
  await assign(CANTEEN_USER, "canteen", [c1]);
  await assign(ADMIN_USER, "admin", [c1]);
  console.log(`setup: done c1=${c1} c2=${c2}`);
}

main().catch((err) => {
  console.error(err.message ?? err);
  process.exit(1);
});
