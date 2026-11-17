<script setup lang="ts">
import {
  Archive,
  ChevronRight,
  Forward,
  Inbox,
  Mail,
  MailOpen,
  Reply,
  ReplyAll,
  Star,
  Tag,
  Trash2,
} from "@lucide/vue";
import { onClickOutside, onKeyStroke } from "@vueuse/core";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import type { ComposeKind } from "../../types/mail";

const props = defineProps<{
  x: number;
  y: number;
  subject: string;
  count: number;
  draft: boolean;
  filingLabel: string;
  filingTarget: string;
  starLabel: string;
  readActions: { read: boolean; label: string }[];
  folders: { id: string; name: string }[];
  labels: { id: string; name: string }[];
}>();

const emit = defineEmits<{
  close: [];
  open: [];
  reply: [kind: ComposeKind];
  read: [read: boolean];
  star: [];
  file: [];
  move: [target: string];
  label: [id: string];
  remove: [];
}>();

const panel = ref<HTMLElement | null>(null);
const left = ref(props.x);
const top = ref(props.y);
const section = ref<"move" | "label" | null>(null);

const places = computed(() => [
  { id: "inbox", name: "Inbox" },
  { id: "archive", name: "Archive" },
  { id: "spam", name: "Spam" },
  { id: "trash", name: "Trash" },
  ...props.folders,
]);

const caption = computed(() => (props.count > 1 ? `${props.count} selected` : props.subject));

const itemClass =
  "flex w-full items-center gap-2 rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas";

function place() {
  const el = panel.value;
  if (!el) {
    return;
  }
  const rect = el.getBoundingClientRect();
  const pad = 8;
  left.value = Math.max(pad, Math.min(props.x, window.innerWidth - rect.width - pad));
  top.value = Math.max(pad, Math.min(props.y, window.innerHeight - rect.height - pad));
}

function toggle(next: "move" | "label") {
  section.value = section.value === next ? null : next;
}

function focusItem(step: number) {
  const items = panel.value?.querySelectorAll<HTMLButtonElement>("[role='menuitem']");
  if (!items || items.length === 0) {
    return;
  }
  const list = [...items];
  const current = list.findIndex((item) => item === document.activeElement);
  const next = current === -1 ? 0 : (current + step + list.length) % list.length;
  list[next]?.focus();
}

onClickOutside(panel, () => emit("close"));
onKeyStroke("Escape", () => emit("close"));

function onKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    focusItem(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    focusItem(-1);
  }
}

onMounted(async () => {
  await nextTick();
  place();
  panel.value?.focus();
});

watch(section, async () => {
  await nextTick();
  place();
});
</script>

<template>
  <Teleport to="body">
    <div
      ref="panel"
      tabindex="-1"
      role="menu"
      aria-label="Message actions"
      class="km-card fixed z-50 w-60 p-1.5 text-ink outline-none"
      :style="{ left: `${left}px`, top: `${top}px` }"
      @keydown="onKeydown"
      @contextmenu.prevent
    >
      <p class="truncate px-2.5 py-1.5 text-xs text-muted">{{ caption }}</p>
      <button type="button" role="menuitem" :class="itemClass" @click="emit('open')">Open</button>
      <template v-if="!draft">
        <button type="button" role="menuitem" :class="itemClass" @click="emit('reply', 'reply')">
          <Reply class="size-4 text-muted" />
          Reply
        </button>
        <button type="button" role="menuitem" :class="itemClass" @click="emit('reply', 'reply-all')">
          <ReplyAll class="size-4 text-muted" />
          Reply all
        </button>
        <button type="button" role="menuitem" :class="itemClass" @click="emit('reply', 'forward')">
          <Forward class="size-4 text-muted" />
          Forward
        </button>
      </template>
      <div class="my-1 h-px bg-line" />
      <button
        v-for="action in readActions"
        :key="action.label"
        type="button"
        role="menuitem"
        :class="itemClass"
        @click="emit('read', action.read)"
      >
        <MailOpen v-if="action.read" class="size-4 text-muted" />
        <Mail v-else class="size-4 text-muted" />
        {{ action.label }}
      </button>
      <button type="button" role="menuitem" :class="itemClass" @click="emit('star')">
        <Star class="size-4 text-muted" />
        {{ starLabel }}
      </button>
      <div class="my-1 h-px bg-line" />
      <button type="button" role="menuitem" :class="itemClass" @click="emit('file')">
        <Inbox v-if="filingTarget === 'inbox'" class="size-4 text-muted" />
        <Archive v-else class="size-4 text-muted" />
        {{ filingLabel }}
      </button>
      <button
        type="button"
        role="menuitem"
        :class="itemClass"
        :aria-expanded="section === 'move'"
        @click="toggle('move')"
      >
        Move
        <ChevronRight class="ml-auto size-3.5 text-muted" :class="section === 'move' ? 'rotate-90' : ''" />
      </button>
      <div v-if="section === 'move'">
        <button
          v-for="place in places"
          :key="place.id"
          type="button"
          role="menuitem"
          class="flex w-full items-center rounded-[8px] py-2 pr-2.5 pl-8 text-left text-sm hover:bg-canvas"
          @click="emit('move', place.id)"
        >
          {{ place.name }}
        </button>
      </div>
      <button
        type="button"
        role="menuitem"
        :class="itemClass"
        :aria-expanded="section === 'label'"
        @click="toggle('label')"
      >
        <Tag class="size-4 text-muted" />
        Label
        <ChevronRight class="ml-auto size-3.5 text-muted" :class="section === 'label' ? 'rotate-90' : ''" />
      </button>
      <div v-if="section === 'label'">
        <p v-if="labels.length === 0" class="px-8 py-2 text-sm text-muted">No labels yet.</p>
        <button
          v-for="label in labels"
          :key="label.id"
          type="button"
          role="menuitem"
          class="flex w-full items-center rounded-[8px] py-2 pr-2.5 pl-8 text-left text-sm hover:bg-canvas"
          @click="emit('label', label.id)"
        >
          {{ label.name }}
        </button>
      </div>
      <div class="my-1 h-px bg-line" />
      <button type="button" role="menuitem" :class="[itemClass, 'text-danger']" @click="emit('remove')">
        <Trash2 class="size-4" />
        Delete
      </button>
    </div>
  </Teleport>
</template>
