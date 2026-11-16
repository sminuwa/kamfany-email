import { usePreferredDark } from "@vueuse/core";
import { computed, watch } from "vue";
import { usePreferencesStore } from "../stores/preferences";
import { applyTheme, resolveTheme } from "../utils/theme";

export function useTheme() {
  const preferences = usePreferencesStore();
  const systemDark = usePreferredDark();
  const resolved = computed(() =>
    resolveTheme(preferences.preference, systemDark.value),
  );

  watch(
    resolved,
    (theme) => {
      applyTheme(theme);
    },
    { immediate: true },
  );

  return { resolved };
}
