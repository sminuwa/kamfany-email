import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { THEME_STORAGE_KEY } from "../utils/theme";
import { usePreferencesStore } from "./preferences";

describe("preferences store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("stores a theme choice", () => {
    const store = usePreferencesStore();
    store.setTheme("dark");
    expect(store.preference).toBe("dark");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("dark");
  });

  it("reads a stored choice", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "light");
    const store = usePreferencesStore();
    expect(store.preference).toBe("light");
  });
});
