<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api } from "../api";

interface Device {
  id: string;
  friendly_name: string;
  user_agent: string;
  ip: string;
  session_active: boolean;
}
interface Passkey { id: string; friendly_name: string }
interface Approval {
  id: string;
  user_agent: string;
  friendly_name: string;
  ip: string;
  status: string;
}

const devices = ref<Device[]>([]);
const passkeys = ref<Passkey[]>([]);
const pending = ref<Approval[]>([]);

async function load() {
  const body = await api<{ devices: Device[]; passkeys: Passkey[] }>("/api/devices");
  devices.value = body.devices;
  passkeys.value = body.passkeys;
  const approvals = await api<{ items: Approval[] }>("/api/login-approvals/pending");
  pending.value = approvals.items;
}

async function kick(id: string) {
  await api(`/api/devices/${id}`, { method: "DELETE" });
  ElMessage.success("已踢下线");
  await load();
}

async function unbind(id: string) {
  await api(`/api/passkeys/${id}`, { method: "DELETE" });
  await load();
}

async function decide(id: string, allow: boolean) {
  await api(`/api/login-approvals/${id}/${allow ? "allow" : "deny"}`, { method: "POST" });
  ElMessage.success(allow ? "已允许，对方可以绑定 Passkey" : "已拒绝");
  await load();
}

onMounted(load);
</script>

<template>
  <div>
    <h1 class="page-title">设备与多账号</h1>
    <el-alert type="warning" show-icon :closable="false" class="mb" title="只管理本账号设备；不能按食堂批准他人的新设备" />
    <div class="card">
      <el-table :data="devices">
        <el-table-column prop="friendly_name" label="友好名称" />
        <el-table-column prop="user_agent" label="UA" min-width="220" />
        <el-table-column prop="ip" label="IP" width="140" />
        <el-table-column label="会话" width="100">
          <template #default="{ row }"><span :class="{ on: row.session_active }">{{ row.session_active ? "在线" : "已结束" }}</span></template>
        </el-table-column>
        <el-table-column label="操作" width="120">
          <template #default="{ row }"><el-button type="danger" size="small" @click="kick(row.id)">踢下线</el-button></template>
        </el-table-column>
      </el-table>
    </div>
    <h2>Passkey</h2>
    <div v-for="item in passkeys" :key="item.id" class="card row">
      <span>{{ item.friendly_name || "Passkey" }}</span>
      <el-button type="danger" plain size="small" @click="unbind(item.id)">解绑</el-button>
    </div>
    <p v-if="passkeys.length === 0" class="muted">还没有绑定 Passkey</p>
    <div v-for="item in pending" :key="item.id" class="card pending">
      <h3>待确认 · {{ item.friendly_name }}</h3>
      <div class="muted">IP：{{ item.ip }}</div>
      <div class="muted">UA：{{ item.user_agent }}</div>
      <div class="actions">
        <el-button type="primary" @click="decide(item.id, true)">允许</el-button>
        <el-button type="danger" @click="decide(item.id, false)">拒绝</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mb { margin-bottom: 16px; }
.card { overflow: hidden; margin-bottom: 12px; }
.row, .actions { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
.row, .pending { padding: 14px 16px; }
.on { color: #2d8a45; }
.pending { max-width: 420px; }
.pending h3 { margin: 0 0 8px; }
</style>
