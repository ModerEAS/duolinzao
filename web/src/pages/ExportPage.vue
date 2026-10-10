<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";
import { apiBlob, explain } from "../api";

const files = [
  ["canteens.json", "食堂"],
  ["dishes.json", "菜谱"],
  ["images/", "菜品图片"],
  ["meal_orders.json", "报餐"],
  ["scores.json", "评分"],
  ["comments.json", "评语"],
  ["audit.json", "审计"],
];
const busy = ref(false);

async function download() {
  busy.value = true;
  try {
    const blob = await apiBlob("/api/admin/exports", { method: "POST" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "duolinzao-export.zip";
    link.click();
    URL.revokeObjectURL(url);
    ElMessage.success("已开始下载");
  } catch (error) {
    ElMessage.error(explain(error));
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div>
    <h1 class="page-title">数据导出</h1>
    <el-alert type="info" show-icon :closable="false" class="mb" title="每次导出同一份压缩包，页面上的清单不会改成部分导出。" />
    <div class="card list">
      <div v-for="[name, label] in files" :key="name" class="row">
        <strong>{{ label }}</strong>
        <span class="muted">{{ name }}</span>
      </div>
      <p class="muted">不含密码哈希、会话令牌、邀请码原文和渠道密钥。</p>
      <el-button type="primary" :loading="busy" @click="download">导出并下载</el-button>
    </div>
  </div>
</template>

<style scoped>
.mb { margin-bottom: 16px; }
.list { padding: 18px; display: flex; flex-direction: column; gap: 10px; }
.row { display: flex; justify-content: space-between; border-bottom: 1px solid #f3f4f6; padding-bottom: 8px; }
</style>
