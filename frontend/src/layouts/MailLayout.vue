<script setup lang="ts">
import { onKeyStroke } from "@vueuse/core";
import { onMounted, watch } from "vue";
import { RouterView, useRoute } from "vue-router";
import AppSidebar from "../components/layout/AppSidebar.vue";
import AppTopbar from "../components/layout/AppTopbar.vue";
import { useDeskSearch } from "../composables/useDeskSearch";
import { useLiveDesk } from "../composables/useLiveDesk";
import { useRailCollapsed } from "../composables/useRail";
import { useMailboxStore } from "../stores/mailboxes";
import { useMailStore } from "../stores/messages";
import { useServiceStore } from "../stores/service";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const service = useServiceStore();
const boxes = useMailboxStore();
const mail = useMailStore();
const route = useRoute();
const { railCollapsed } = useRailCollapsed();
useDeskSearch();
useLiveDesk();

watch(
  () =>
    [
      boxes.active?.id ?? "",
      typeof route.meta.folder === "string" ? route.meta.folder : "",
      typeof route.params.folderId === "string" ? route.params.folderId : "",
    ].join("|"),
  () => {
    const mailbox = boxes.active;
    if (!mailbox?.canRead) {
      mail.clearDesk();
      return;
    }
    if (route.name === "contacts") {
      void mail.ensureFolders({
        id: mailbox.id,
        address: mailbox.address,
        displayName: mailbox.displayName,
      });
      return;
    }
    const folderId = typeof route.params.folderId === "string" ? route.params.folderId : null;
    const view = typeof route.meta.folder === "string" ? route.meta.folder : "inbox";
    void mail.load(
      { id: mailbox.id, address: mailbox.address, displayName: mailbox.displayName },
      folderId ? "custom" : view,
      folderId,
    );
  },
  { immediate: true },
);

onMounted(() => {
  void service.refresh();
});

watch(
  () => route.path,
  () => {
    ui.closeMobile();
  },
);

onKeyStroke("k", (event) => {
  if (!(event.metaKey || event.ctrlKey)) {
    return;
  }
  event.preventDefault();
  document.getElementById("mail-search")?.focus();
});
</script>

<template>
  <div
    class="mail-shell h-dvh overflow-hidden bg-canvas text-ink min-[960px]:grid min-[960px]:transition-[grid-template-columns] min-[960px]:duration-200 min-[960px]:ease-out"
    :class="
      railCollapsed
        ? 'min-[960px]:grid-cols-[72px_minmax(0,1fr)]'
        : 'min-[960px]:grid-cols-[248px_minmax(0,1fr)]'
    "
  >
    <a
      href="#mail-desk"
      class="desk-chrome sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-50 focus:rounded-control focus:bg-surface focus:px-3 focus:py-2"
    >
      Skip to messages
    </a>
    <button
      v-if="ui.mobileOpen"
      type="button"
      class="desk-chrome fixed inset-0 z-20 bg-ink/40 min-[960px]:hidden"
      aria-label="Close menu"
      @click="ui.closeMobile()"
    />
    <AppSidebar />
    <div class="mail-column flex h-full min-h-0 min-w-0 flex-col">
      <AppTopbar />
      <main id="mail-desk" class="min-h-0 flex-1">
        <RouterView />
      </main>
    </div>
  </div>
</template>
