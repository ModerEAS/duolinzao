import { ref } from "vue";
import { api } from "./api";

// 页脚版本：启动时从 /api/health 拉一次缓存，三处页脚同一文案
// `Powered by 多邻灶 · Version: <ver>`。
export const appVersion = ref("");

let loaded = false;

export async function loadVersion(): Promise<void> {
  if (loaded) return;
  loaded = true;
  try {
    const body = await api<{ version?: string }>("/api/health");
    if (body.version) appVersion.value = body.version;
  } catch {
    /* 健康接口不可用时页脚留空，不挡登录 */
  }
}

export function versionFooter(): string {
  return appVersion.value ? `Powered by 多邻灶 · Version: ${appVersion.value}` : "";
}
