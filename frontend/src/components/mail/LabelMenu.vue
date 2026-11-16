<script setup lang="ts">
import { Tag } from "@lucide/vue";
import { ref } from "vue";
import { useMailStore } from "../../stores/messages";
import MenuPanel from "../common/MenuPanel.vue";

withDefaults(
  defineProps<{
    compact?: boolean;
  }>(),
  { compact: false },
);

const emit = defineEmits<{
  label: [id: string];
}>();

const mail = useMailStore();
const open = ref(false);

function choose(id: string) {
  emit("label", id);
  open.value = false;
}
</script>

<template>
  <MenuPanel v-model="open" label="Add label" :align="compact ? 'end' : 'start'">
    <template #trigger="{ toggle, open: isOpen }">
      <button
        type="button"
        class="rounded-control text-sm text-ink hover:bg-canvas"
        :class="compact ? 'grid size-9 place-items-center' : 'inline-flex h-8 items-center gap-1.5 px-2'"
        :aria-expanded="isOpen"
        aria-haspopup="menu"
        aria-label="Add label"
        @click="toggle"
      >
        <Tag class="size-4" />
        <span v-if="!compact">Label</span>
      </button>
    </template>
    <p v-if="mail.labels.length === 0" class="px-2.5 py-2 text-sm text-muted">No labels yet.</p>
    <button
      v-for="label in mail.labels"
      :key="label.id"
      type="button"
      role="menuitem"
      class="flex w-full items-center rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      @click="choose(label.id)"
    >
      {{ label.name }}
    </button>
  </MenuPanel>
</template>
