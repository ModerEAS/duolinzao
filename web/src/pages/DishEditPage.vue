<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { api, explain, KINDS, type Dish, type Groups, type Kind } from "../api";
import { auth, managedCanteens } from "../session";

const props = defineProps<{ dishId: string | null }>();
const emit = defineEmits<{ back: [] }>();

const name = ref("");
const kind = ref<Kind>("菜");
const tags = ref<string[]>([]);
const tagInput = ref("");
const enabled = ref(true);
const imageUrl = ref<string | null>(null);
const file = ref<File | null>(null);
const canteenId = ref("");
const originCanteen = ref("");
const creating = computed(() => !props.dishId);
const canteens = computed(() => managedCanteens());

async function load() {
  file.value = null;
  tagInput.value = "";
  if (!props.dishId || !auth.canteenId) {
    name.value = "";
    kind.value = "菜";
    tags.value = [];
    enabled.value = true;
    imageUrl.value = null;
    canteenId.value = auth.canteenId;
    originCanteen.value = auth.canteenId;
    return;
  }
  const body = await api<{ groups: Groups<Dish> }>(`/api/canteens/${auth.canteenId}/dishes`);
  const found = [...body.groups.菜, ...body.groups.饭, ...body.groups.汤].find((item) => item.id === props.dishId);
  if (!found) return;
  name.value = found.name;
  kind.value = found.kind;
  tags.value = [...found.taste_tags];
  enabled.value = found.enabled;
  imageUrl.value = found.image_url;
  canteenId.value = found.canteen_id;
  originCanteen.value = found.canteen_id;
}

function addTag() {
  const text = tagInput.value.trim();
  if (text && !tags.value.includes(text)) tags.value.push(text);
  tagInput.value = "";
}

function onFile(upload: { raw?: File }) {
  file.value = upload.raw ?? null;
}

async function save() {
  if (!canteenId.value) return;
  const payload = {
    name: name.value,
    kind: kind.value,
    taste_tags: tags.value,
    enabled: enabled.value,
    canteen_id: canteenId.value,
  };
  const path = creating.value
    ? `/api/canteens/${canteenId.value}/dishes`
    : `/api/canteens/${originCanteen.value}/dishes/${props.dishId}`;
  try {
    const saved = await api<Dish>(path, { method: creating.value ? "POST" : "PUT", body: JSON.stringify(payload) });
    if (file.value) {
      const form = new FormData();
      form.append("file", file.value);
      await api(`/api/canteens/${saved.canteen_id}/dishes/${saved.id}/image`, { method: "POST", body: form });
    }
    auth.canteenId = saved.canteen_id;
    ElMessage.success("已保存");
    emit("back");
  } catch (error) {
    ElMessage.error(explain(error));
  }
}

async function removeImage() {
  if (!props.dishId || !originCanteen.value) return;
  await api(`/api/canteens/${originCanteen.value}/dishes/${props.dishId}/image`, { method: "DELETE" });
  imageUrl.value = null;
  file.value = null;
}

async function disable() {
  try {
    await ElMessageBox.confirm("须显式确认；强提醒不会自动停用。确定停用该菜？", "停用确认", {
      type: "warning",
      confirmButtonText: "确认停用",
    });
  } catch {
    return;
  }
  enabled.value = false;
  await save();
}

watch(() => [props.dishId, auth.canteenId], load, { immediate: true });
</script>

<template>
  <div>
    <div class="crumb">菜谱 / {{ creating ? "新建菜" : "编辑菜" }}</div>
    <div v-if="!creating" class="hero card">
      <img v-if="imageUrl" :src="imageUrl" alt="" class="hero-img" />
      <div v-else class="hero-img ph" />
      <div>
        <h2>{{ name || "未命名" }}</h2>
        <div class="tags">
          <el-tag v-for="tag in tags" :key="tag" size="small" effect="plain" round>{{ tag }}</el-tag>
        </div>
        <div class="tags" style="margin-top: 10px">
          <el-upload :auto-upload="false" :show-file-list="false" :on-change="onFile">
            <el-button size="small">更换图片</el-button>
          </el-upload>
          <el-button v-if="imageUrl" size="small" @click="removeImage">移除图片</el-button>
        </div>
      </div>
    </div>
    <div v-else class="card form-card">
      <el-upload drag :auto-upload="false" :show-file-list="false" :on-change="onFile">
        <div class="up-hint">上传菜品图片<br /><span>可以不上传，稍后再补</span></div>
      </el-upload>
    </div>

    <div class="card form-card">
      <el-form label-position="top" style="max-width: 520px">
        <el-form-item label="所属食堂">
          <el-select v-model="canteenId" style="width: 100%" placeholder="选择食堂">
            <el-option v-for="item in canteens" :key="item.id" :label="item.name" :value="item.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="name" :placeholder="creating ? '请输入菜品名称' : undefined" />
        </el-form-item>
        <el-form-item label="类别" required>
          <el-radio-group v-model="kind">
            <el-radio-button v-for="item in KINDS" :key="item" :value="item">{{ item }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="口味标签">
          <div class="tag-edit">
            <el-tag v-for="tag in tags" :key="tag" closable @close="tags = tags.filter((item) => item !== tag)">{{ tag }}</el-tag>
            <el-input v-model="tagInput" size="small" placeholder="添加" style="width: 120px" @keyup.enter="addTag" />
            <el-button size="small" text type="primary" @click="addTag">+ 添加</el-button>
          </div>
        </el-form-item>
        <el-form-item v-if="creating" label="启用">
          <el-switch v-model="enabled" />
          <span class="hint">默认启用</span>
        </el-form-item>
      </el-form>
      <div v-if="!creating" class="status-row">
        <span class="on">当前{{ enabled ? "已启用" : "已停用" }}</span>
        <el-button v-if="enabled" type="danger" plain @click="disable">停用</el-button>
        <el-button v-else type="primary" plain @click="enabled = true">启用</el-button>
        <div class="warn-note">须显式确认；强提醒不会自动停用</div>
      </div>
      <div class="actions">
        <el-button type="primary" @click="save">保存</el-button>
        <el-button @click="emit('back')">取消</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.crumb { color: #6b7280; font-size: 13px; margin-bottom: 8px; }
.card { padding: 18px; margin-bottom: 14px; }
.hero { display: flex; gap: 16px; }
.hero-img { width: 96px; height: 96px; border-radius: 12px; object-fit: cover; }
.hero h2 { margin: 0 0 8px; font-size: 20px; }
.tags, .tag-edit, .actions, .status-row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.status-row { margin: 8px 0 16px; }
.on { color: #2d8a45; font-weight: 600; }
.warn-note { width: 100%; color: #e53935; font-size: 12px; }
.hint { margin-left: 10px; color: #9ca3af; font-size: 12px; }
.up-hint { color: #6b7280; line-height: 1.6; }
.up-hint span { font-size: 12px; color: #9ca3af; }
</style>
