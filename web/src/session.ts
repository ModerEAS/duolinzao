import { reactive } from "vue";
import { api, setUnauthorized, type Canteen, type Me } from "./api";

export const auth = reactive({
  me: null as Me | null,
  canteens: [] as Canteen[],
  canteenId: "",
  ready: false,
});

export function can(code: string): boolean {
  return auth.me?.permissions.includes(code) ?? false;
}

export function managedCanteens(): Canteen[] {
  if (!auth.me) return [];
  if (auth.me.roles.some((role) => role.role === "super_admin")) return auth.canteens;
  const ids = new Set(auth.me.roles.flatMap((role) => role.canteen_ids));
  return auth.canteens.filter((item) => ids.has(item.id));
}

export async function refreshSession(): Promise<void> {
  try {
    auth.me = await api<Me>("/api/me");
    const listed = await api<{ items: Canteen[] }>("/api/canteens");
    auth.canteens = listed.items;
    const scoped = managedCanteens();
    if (!scoped.some((item) => item.id === auth.canteenId)) {
      auth.canteenId = scoped[0]?.id ?? "";
    }
  } catch {
    auth.me = null;
  } finally {
    auth.ready = true;
  }
}

setUnauthorized(() => {
  auth.me = null;
});
