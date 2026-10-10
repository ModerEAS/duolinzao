<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api, explain } from "../api";

const enabled = ref(false);
const ready = ref(false);

async function load() {
  const body = await api<{ enabled: boolean }>("/api/admin/settings/email");
  enabled.value = body.enabled;
  ready.value = true;
}

async function save(value: string | number | boolean) {
  const next = value === true;
  try {
    const body = await api<{ enabled: boolean }>("/api/admin/settings/email", {
      method: "PUT",
      body: JSON.stringify({ enabled: next }),
    });
    enabled.value = body.enabled;
    ElMessage.success(body.enabled ? "邮箱能力已开启" : "邮箱能力已关闭");
  } catch (error) {
    enabled.value = !next;
    ElMessage.error(explain(error));
  }
}

onMounted(load);
</script>

<template>
  <div>
    <h1 class="page-title">邮箱设置</h1>
    <div class="card box">
      <div class="row">
        <div>
          <strong>邮箱能力</strong>
          <p class="muted">关闭后登录页不出现 magic link，也不强制验证邮箱。</p>
        </div>
        <el-switch v-if="ready" :model-value="enabled" @change="save" />
      </div>
      <el-alert type="info" show-icon :closable="false" title="邮件服务器、发件人和端口只在配置文件里。这个页面不回显、也不写入这些值。" />
    </div>
  </div>
</template>

<style scoped>
.box { padding: 18px; display: flex; flex-direction: column; gap: 16px; }
.row { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
</style>
