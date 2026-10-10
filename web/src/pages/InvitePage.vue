<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api } from "../api";

interface Invite {
  id: string;
  created_at: string;
  used_at: string | null;
  used_by: string | null;
}

const rows = ref<Invite[]>([]);
const shown = ref<string | null>(null);
const dialog = ref(false);

async function load() {
  const body = await api<{ items: Invite[] }>("/api/admin/invites");
  rows.value = body.items;
}

async function create() {
  const body = await api<{ code: string }>("/api/admin/invites", { method: "POST" });
  shown.value = body.code;
  dialog.value = true;
  await load();
}

async function copy() {
  if (!shown.value) return;
  await navigator.clipboard.writeText(shown.value);
  ElMessage.success("已复制");
}

onMounted(load);
</script>

<template>
  <div>
    <div class="head">
      <h1 class="page-title">邀请码</h1>
      <el-button type="primary" @click="create">发放邀请码</el-button>
    </div>
    <div class="card">
      <el-table :data="rows">
        <el-table-column label="邀请码">
          <template #default>完整码只在发放时出现一次</template>
        </el-table-column>
        <el-table-column prop="created_at" label="发放时间" />
        <el-table-column label="是否已用" width="140">
          <template #default="{ row }">
            <span :style="{ color: row.used_at ? '#6b7280' : '#2d8a45' }">{{ row.used_at ? `已使用 · ${row.used_by}` : "未使用" }}</span>
          </template>
        </el-table-column>
      </el-table>
    </div>
    <el-dialog v-model="dialog" title="邀请码展示" width="440px" align-center @closed="shown = null">
      <div class="code">{{ shown }}</div>
      <el-alert type="warning" :closable="false" show-icon title="完整码仅显示一次，离开后列表不再显示全文" />
      <template #footer>
        <el-button type="primary" plain @click="copy">复制</el-button>
        <el-button @click="dialog = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.head { display: flex; justify-content: space-between; align-items: center; }
.card { overflow: hidden; }
.code { font-family: ui-monospace, monospace; font-size: 20px; font-weight: 700; text-align: center; padding: 8px 0 16px; word-break: break-all; }
</style>
