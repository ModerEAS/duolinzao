<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Key, Lock, Message } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import { ApiError, api, type Me } from "../api";
import { refreshSession } from "../session";

const emit = defineEmits<{ enter: [] }>();

const methods = ref<string[]>(["passkey", "totp", "password"]);
const emailEnabled = ref(false);
const showPwd = ref(false);
const showTotp = ref(false);
const showMagic = ref(false);
const showWait = ref(false);
const showRegister = ref(false);
const username = ref("");
const password = ref("");
const totpUser = ref("");
const totpCode = ref("");
const magicUser = ref("");
const regUser = ref("");
const regCode = ref("");
const regPassword = ref("");
const wait = ref<{ id: string; poll: string; ip: string; ua: string; name: string; status: string } | null>(null);

onMounted(async () => {
  try {
    const body = await api<{ methods: string[]; email_enabled: boolean }>("/api/session/methods");
    methods.value = body.methods;
    emailEnabled.value = body.email_enabled;
  } catch {
    /* 保持默认顺序 */
  }
  const magic = new URLSearchParams(location.search).get("magic");
  if (magic) {
    try {
      await api("/api/session/magic-link/consume", {
        method: "POST",
        body: JSON.stringify({ token: magic }),
      });
      await refreshSession();
      emit("enter");
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : "链接无效");
    }
  }
});

async function enter() {
  await refreshSession();
  emit("enter");
}

async function passwordLogin() {
  try {
    await api("/api/session/password", {
      method: "POST",
      body: JSON.stringify({ username: username.value, password: password.value }),
    });
    await enter();
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "登录失败");
  }
}

async function totpLogin() {
  try {
    await api("/api/session/totp", {
      method: "POST",
      body: JSON.stringify({ username: totpUser.value, code: totpCode.value }),
    });
    await enter();
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "验证码不正确");
  }
}

async function requestMagic() {
  try {
    await api("/api/session/magic-link/request", {
      method: "POST",
      body: JSON.stringify({ username: magicUser.value }),
    });
    ElMessage.success("如果账号已绑定邮箱，登录链接已发出");
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "无法发送");
  }
}

async function passkeyLogin() {
  if (!username.value) {
    showPwd.value = true;
    ElMessage.info("Passkey 需要先填写账号");
    return;
  }
  try {
    const started = await api<{ challenge_id: string; publicKey: PublicKeyCredentialRequestOptions }>(
      "/api/session/passkey/start",
      { method: "POST", body: JSON.stringify({ username: username.value }) },
    );
    const publicKey = prepareRequest(started.publicKey);
    const credential = (await navigator.credentials.get({ publicKey })) as PublicKeyCredential | null;
    if (!credential) return;
    await api("/api/session/passkey/finish", {
      method: "POST",
      body: JSON.stringify({
        username: username.value,
        challenge_id: started.challenge_id,
        credential: credentialToJson(credential),
      }),
    });
    await enter();
  } catch (error) {
    if (error instanceof ApiError && error.code === "passkey_none") {
      showPwd.value = true;
      ElMessage.info("这个账号还没有 Passkey，请用下面的密码登录");
      return;
    }
    ElMessage.error(error instanceof Error ? error.message : "Passkey 登录失败");
  }
}

function prepareRequest(options: PublicKeyCredentialRequestOptions): PublicKeyCredentialRequestOptions {
  const source = options as PublicKeyCredentialRequestOptions & { challenge: string };
  return {
    ...options,
    challenge: toBuffer(source.challenge),
    allowCredentials: options.allowCredentials?.map((item) => ({
      ...item,
      id: typeof item.id === "string" ? toBuffer(item.id) : item.id,
    })),
  };
}

function toBuffer(value: string | ArrayBuffer): ArrayBuffer {
  if (value instanceof ArrayBuffer) return value;
  const padded = value.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(padded + "=".repeat((4 - (padded.length % 4)) % 4));
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
}

function credentialToJson(credential: PublicKeyCredential) {
  const response = credential.response as AuthenticatorAssertionResponse;
  return {
    id: credential.id,
    rawId: bufferToBase64url(credential.rawId),
    type: credential.type,
    response: {
      clientDataJSON: bufferToBase64url(response.clientDataJSON),
      authenticatorData: bufferToBase64url(response.authenticatorData),
      signature: bufferToBase64url(response.signature),
      userHandle: response.userHandle ? bufferToBase64url(response.userHandle) : null,
    },
  };
}

function bufferToBase64url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let bin = "";
  bytes.forEach((byte) => {
    bin += String.fromCharCode(byte);
  });
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

async function register() {
  try {
    await api("/api/register", {
      method: "POST",
      body: JSON.stringify({
        username: regUser.value,
        invite_code: regCode.value,
        password: regPassword.value,
      }),
    });
    username.value = regUser.value;
    password.value = regPassword.value;
    await passwordLogin();
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "注册失败");
  }
}

async function requestApproval() {
  try {
    const created = await api<{
      id: string;
      poll_token: string;
      status: string;
      ip: string;
      user_agent: string;
      friendly_name: string;
    }>("/api/login-approvals", {
      method: "POST",
      body: JSON.stringify({ username: username.value }),
    });
    wait.value = {
      id: created.id,
      poll: created.poll_token,
      ip: created.ip,
      ua: created.user_agent,
      name: created.friendly_name,
      status: created.status,
    };
    poll();
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "无法发起确认");
  }
}

async function poll() {
  if (!wait.value) return;
  try {
    const body = await api<{ status: string }>(
      `/api/login-approvals/${wait.value.id}?poll_token=${encodeURIComponent(wait.value.poll)}`,
    );
    wait.value.status = body.status;
    if (body.status === "allowed") {
      const me = await api<Me>("/api/me");
      if (me.id) await enter();
      return;
    }
    if (body.status === "pending") setTimeout(poll, 2000);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "确认失败");
  }
}
</script>

<template>
  <div class="login-wrap">
    <div class="card">
      <div class="brand">
        <span class="brand-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="32" height="32" fill="none">
            <path d="M12 3c-1.8 0-3.3 1-4.1 2.4A3.5 3.5 0 0 0 4 8.8C4 10.8 5.6 12.4 7.6 12.4h8.8c2 0 3.6-1.6 3.6-3.6a3.5 3.5 0 0 0-3.9-3.4A4.6 4.6 0 0 0 12 3Z" fill="#2d8a45"/>
            <rect x="7" y="12.2" width="10" height="7.3" rx="1.2" fill="#2d8a45"/>
            <rect x="6.2" y="18.8" width="11.6" height="1.8" rx="0.6" fill="#246e37"/>
          </svg>
        </span>
        <div>
          <div class="name">多邻灶</div>
          <div class="role">食堂 / 管理横屏工作台</div>
        </div>
      </div>
      <h1>{{ showRegister ? "注册" : "登录" }}</h1>

      <template v-if="!showRegister">
        <el-input v-model="username" placeholder="账号" class="mb" />
        <el-button type="primary" size="large" class="full" @click="passkeyLogin">
          <el-icon><Key /></el-icon>
          使用 Passkey 登录
        </el-button>
        <el-button size="large" class="full outline" @click="showTotp = !showTotp">使用 TOTP</el-button>
        <div v-if="showTotp" class="pwd-box">
          <el-input v-model="totpUser" placeholder="账号" class="mb" />
          <el-input v-model="totpCode" placeholder="6 位验证码" class="mb" />
          <el-button type="primary" class="full" @click="totpLogin">验证并登录</el-button>
        </div>
        <template v-if="emailEnabled && methods.includes('magic_link')">
          <el-button size="large" class="full outline" @click="showMagic = !showMagic">
            <el-icon><Message /></el-icon>
            邮箱 magic link
          </el-button>
          <div v-if="showMagic" class="pwd-box">
            <el-input v-model="magicUser" placeholder="账号" class="mb" />
            <el-button class="full" @click="requestMagic">发送登录链接</el-button>
          </div>
        </template>
        <p v-else class="hint">邮箱能力关闭，不显示 magic link</p>

        <button type="button" class="pwd-toggle" @click="showPwd = !showPwd">
          <span><el-icon><Lock /></el-icon> 密码登录（兜底）</span>
          <span>{{ showPwd ? "▴" : "▾" }}</span>
        </button>
        <div v-if="showPwd" class="pwd-box">
          <el-input v-model="password" type="password" show-password placeholder="密码" class="mb" />
          <el-button type="primary" class="full" @click="passwordLogin">登录</el-button>
        </div>

        <button type="button" class="pwd-toggle" @click="showWait = !showWait">
          <span>新设备等待页内确认</span>
          <span>{{ showWait ? "▴" : "▾" }}</span>
        </button>
        <div v-if="showWait" class="pwd-box">
          <el-button class="full" @click="requestApproval">请已登录设备确认</el-button>
          <p v-if="wait" class="hint">{{ wait.name }} · {{ wait.ip }} · {{ wait.status === "pending" ? "等待确认" : wait.status }}</p>
        </div>
        <button type="button" class="link" @click="showRegister = true">有邀请码，去注册</button>
      </template>

      <template v-else>
        <el-input v-model="regUser" placeholder="账号" class="mb" />
        <el-input v-model="regCode" placeholder="邀请码" class="mb" />
        <el-input v-model="regPassword" type="password" show-password placeholder="密码（至少 8 位）" class="mb" />
        <el-button type="primary" class="full" @click="register">注册并登录</el-button>
        <button type="button" class="link" @click="showRegister = false">返回登录</button>
      </template>
    </div>
    <p class="foot">新设备将等待已登录设备页内确认 · 不扫码</p>
  </div>
</template>

<style scoped>
.login-wrap {
  min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center;
  background: #f3f5f7; padding: 24px;
}
.card {
  width: 100%; max-width: 420px; background: #fff; border-radius: 16px; padding: 28px 28px 22px;
  border: 1px solid #e8eaed; box-shadow: 0 8px 28px rgba(16,24,40,.08);
}
.brand { display: flex; align-items: center; gap: 10px; margin-bottom: 22px; }
.name { font-size: 22px; font-weight: 700; }
.role { font-size: 12px; color: #9ca3af; }
h1 { margin: 0 0 20px; font-size: 28px; text-align: center; font-weight: 700; }
.full { width: 100%; margin: 0 0 10px; }
.outline { --el-button-border-color: #2d8a45; --el-button-text-color: #2d8a45; }
.hint { margin: -2px 0 14px; font-size: 12px; color: #9ca3af; text-align: center; }
.pwd-toggle {
  width: 100%; display: flex; justify-content: space-between; align-items: center;
  border: none; background: transparent; padding: 10px 0 0; border-top: 1px solid #f0f1f3;
  color: #4b5563; cursor: pointer; font-size: 14px;
}
.pwd-box { margin-top: 12px; }
.mb { margin-bottom: 10px; }
.foot { margin-top: 16px; font-size: 12px; color: #9ca3af; }
.link { display: block; width: 100%; margin-top: 14px; border: none; background: transparent; color: #2d8a45; cursor: pointer; }
</style>
