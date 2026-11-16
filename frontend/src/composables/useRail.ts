import { useMediaQuery } from "@vueuse/core";
import { computed } from "vue";
import { useUiStore } from "../stores/ui";

export const DESK_BREAKPOINT = "(min-width: 960px)";

export function useRailCollapsed() {
  const ui = useUiStore();
  const isDesktop = useMediaQuery(DESK_BREAKPOINT);
  const railCollapsed = computed(() => isDesktop.value && ui.collapsed);
  return { isDesktop, railCollapsed };
}
