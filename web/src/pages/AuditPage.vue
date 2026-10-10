<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api, explain } from "../api";

interface AuditRow {
  id: string;
  created_at: string;
  actor_username: string | null;
  role: string | null;
  scope: string;
  action: string;
  object_type: string | null;
  object_id: string | null;
  result: string;
}

const rows = ref<AuditRow[]>([]);

function when(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

async function load() {
  try {
    const body = await api<{ items: AuditRow[] }>("/api/admin/audit");
    rows.value = body.items;
  } catch (error) {
    ElMessage.error(explain(error));
  }
}

onMounted(load);
</script>

<template>
  <div>
    <h1 class="page-title">审计日志</h1>
    <p class="muted">保留不少于 6 个月。本页不能删除未满期记录。</p>
    <div class="card">
      <el-table :data="rows">
        <el-table-column label="时间" min-width="160">
          <template #default="{ row }">{{ when(row.created_at) }}</template>
        </el-table-column>
        <el-table-column prop="actor_username" label="操作者" width="120" />
        <el-table-column prop="role" label="角色" width="120" />
        <el-table-column prop="scope" label="范围" min-width="140" />
        <el-table-column prop="action" label="动作" width="160" />
        <el-table-column label="对象" min-width="180">
          <template #default="{ row }">{{ row.object_type || "—" }} {{ row.object_id || "" }}</template>
        </el-table-column>
        <el-table-column prop="result" label="结果" width="80" />
      </el-table>
    </div>
  </div>
</template>

<style scoped>
.card { overflow: hidden; }
</style>
