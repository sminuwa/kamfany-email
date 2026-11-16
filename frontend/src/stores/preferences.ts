import { defineStore } from "pinia";
import { ref } from "vue";
import {
  THEME_STORAGE_KEY,
  readThemePreference,
  type ThemePreference,
} from "../utils/theme";

export const usePreferencesStore = defineStore("preferences", () => {
  const preference = ref<ThemePreference>(readThemePreference());

  function setTheme(next: ThemePreference) {
    preference.value = next;
    try {
      localStorage.setItem(THEME_STORAGE_KEY, next);
    } catch {
      // The choice still applies for this visit.
    }
  }

  return { preference, setTheme };
});
