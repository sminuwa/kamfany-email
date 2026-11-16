import { defineStore } from "pinia";
import { ref, watch } from "vue";
import type { ComposeIntent } from "../types/mail";

const SIDEBAR_STORAGE_KEY = "kamfany-mail.sidebar-collapsed";

function readCollapsed(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export const useUiStore = defineStore("ui", () => {
  const collapsed = ref(readCollapsed());
  const mobileOpen = ref(false);
  const composerOpen = ref(false);
  const composeIntent = ref<ComposeIntent>({ kind: "new", messageId: null, draftId: null });
  const searchQuery = ref("");
  const filtersOpen = ref(false);

  watch(collapsed, (value) => {
    try {
      localStorage.setItem(SIDEBAR_STORAGE_KEY, String(value));
    } catch {
      // The rail still collapses for this visit.
    }
  });

  function toggleCollapsed() {
    collapsed.value = !collapsed.value;
  }

  function toggleMobile() {
    mobileOpen.value = !mobileOpen.value;
  }

  function closeMobile() {
    mobileOpen.value = false;
  }

  function openComposer(intent: ComposeIntent = { kind: "new", messageId: null, draftId: null }) {
    composeIntent.value = intent;
    composerOpen.value = true;
    mobileOpen.value = false;
  }

  function closeComposer() {
    composerOpen.value = false;
  }

  return {
    collapsed,
    mobileOpen,
    composerOpen,
    composeIntent,
    searchQuery,
    filtersOpen,
    toggleCollapsed,
    toggleMobile,
    closeMobile,
    openComposer,
    closeComposer,
  };
});
