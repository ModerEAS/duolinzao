import { onBeforeUnmount, ref } from "vue";

// 窄屏断点 900px：与 OverviewPage 卡片堆叠的 @media (max-width: 900px)
// 取同一值，壳切换与卡片单列同时翻转，避免中间态（一边侧栏一边堆叠卡）。
export const NARROW_BREAKPOINT = 900;

export function useNarrow(breakpoint = NARROW_BREAKPOINT) {
  const query =
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia(`(max-width: ${breakpoint}px)`)
      : null;
  const narrow = ref(query?.matches ?? false);
  const onChange = (event: MediaQueryListEvent) => {
    narrow.value = event.matches;
  };
  query?.addEventListener("change", onChange);
  onBeforeUnmount(() => query?.removeEventListener("change", onChange));
  return narrow;
}
