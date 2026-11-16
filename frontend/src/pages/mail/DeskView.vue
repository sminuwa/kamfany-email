<script setup lang="ts">
import { computed, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import BrandMark from "../../components/common/BrandMark.vue";
import ComposerSheet from "../../components/mail/ComposerSheet.vue";
import MessageList from "../../components/mail/MessageList.vue";
import ReadingPane from "../../components/mail/ReadingPane.vue";
import { useRailCollapsed } from "../../composables/useRail";
import { useMailStore } from "../../stores/messages";
import { useUiStore } from "../../stores/ui";

const route = useRoute();
const router = useRouter();
const ui = useUiStore();
const mail = useMailStore();
const { isDesktop } = useRailCollapsed();

const selectedId = computed(() => {
  const value = route.query.m;
  return typeof value === "string" ? value : null;
});

const selected = computed(() =>
  selectedId.value && mail.detail?.id === selectedId.value ? mail.detail : null,
);

const showList = computed(() => isDesktop.value || (!ui.composerOpen && !selected.value));
const showReader = computed(
  () => isDesktop.value || ui.composerOpen || Boolean(selected.value),
);

function openMessage(id: string) {
  ui.closeMobile();
  const message = mail.byId(id);
  if (message?.status === "draft") {
    ui.openComposer({ kind: "new", messageId: null, draftId: id });
    return;
  }
  void router.push({ query: { ...route.query, m: id } });
}

function closeMessage() {
  const query = { ...route.query };
  delete query.m;
  void router.push({ query });
}

watch(
  selectedId,
  (id) => {
    if (id) {
      void mail.focus(id);
      return;
    }
    mail.clearDetail();
  },
  { immediate: true },
);
</script>

<template>
  <div class="desk-grid grid h-full min-h-0 grid-cols-1 gap-3 min-[960px]:grid-cols-[minmax(280px,0.92fr)_minmax(0,1.15fr)]">
    <MessageList v-show="showList" class="rounded-panel shadow-panel" @open="openMessage" />
    <section v-show="showReader" class="flex h-full min-h-0 flex-col rounded-panel bg-surface shadow-panel" aria-label="Reading pane">
      <ComposerSheet v-if="ui.composerOpen" />
      <ReadingPane v-else-if="selected" :message="selected" @close="closeMessage" />
      <div v-else-if="selectedId" class="grid h-full place-items-center px-8 text-center">
        <p class="text-sm text-muted">{{ mail.notice || "Opening the message." }}</p>
      </div>
      <div v-else class="desk-chrome grid h-full min-h-0 flex-1 place-items-center px-8 text-center">
        <div class="max-w-[36ch]">
          <BrandMark class="mx-auto size-14" />
          <p class="mt-5 text-lg font-semibold tracking-[-0.03em]">No message open</p>
          <p class="mt-2 text-sm leading-relaxed text-muted">
            Choose a message from the list to read it here.
          </p>
        </div>
      </div>
    </section>
  </div>
</template>
