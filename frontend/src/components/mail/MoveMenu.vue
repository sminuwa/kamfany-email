<script setup lang="ts">
import { FolderInput } from "@lucide/vue";
import { computed, ref } from "vue";
import { useMailStore } from "../../stores/messages";
import type { OrganizationalFolder } from "../../types/mail";
import MenuPanel from "../common/MenuPanel.vue";

withDefaults(
  defineProps<{
    compact?: boolean;
  }>(),
  { compact: false },
);

const emit = defineEmits<{
  move: [folder: string];
}>();

const mail = useMailStore();
const customFolders = computed(() => mail.folders.filter((folder) => !folder.systemKey));

const open = ref(false);

const targets: { key: OrganizationalFolder; label: string }[] = [
  { key: "inbox", label: "Inbox" },
  { key: "archive", label: "Archive" },
  { key: "spam", label: "Spam" },
  { key: "trash", label: "Trash" },
];

function choose(folder: string) {
  emit("move", folder);
  open.value = false;
}
</script>

<template>
  <MenuPanel v-model="open" label="Move to">
    <template #trigger="{ toggle, open: isOpen }">
      <button
        type="button"
        class="rounded-control text-sm text-ink hover:bg-canvas"
        :class="compact ? 'grid size-9 place-items-center' : 'inline-flex h-8 items-center gap-1.5 px-2'"
        :aria-expanded="isOpen"
        aria-haspopup="menu"
        aria-label="Move"
        @click="toggle"
      >
        <FolderInput class="size-4" />
        <span v-if="!compact">Move</span>
      </button>
    </template>
    <button
      v-for="target in targets"
      :key="target.key"
      type="button"
      role="menuitem"
      class="flex w-full items-center rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      @click="choose(target.key)"
    >
      {{ target.label }}
    </button>
    <button
      v-for="folder in customFolders"
      :key="folder.id"
      type="button"
      role="menuitem"
      class="flex w-full items-center rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      @click="choose(folder.id)"
    >
      {{ folder.name }}
    </button>
  </MenuPanel>
</template>
