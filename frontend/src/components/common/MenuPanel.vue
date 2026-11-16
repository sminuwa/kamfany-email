<script setup lang="ts">
import { onClickOutside, onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";

const open = defineModel<boolean>({ required: true });

withDefaults(
  defineProps<{
    align?: "start" | "end";
    label: string;
  }>(),
  { align: "end" },
);

const root = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);

function toggle() {
  open.value = !open.value;
}

function close() {
  open.value = false;
}

onClickOutside(root, close);
onKeyStroke("Escape", () => {
  if (open.value) {
    close();
  }
});

watch(open, async (isOpen) => {
  if (!isOpen) {
    return;
  }
  await nextTick();
  panel.value?.focus();
});
</script>

<template>
  <div ref="root" class="relative">
    <slot name="trigger" :open="open" :toggle="toggle" />
    <div
      v-if="open"
      ref="panel"
      tabindex="-1"
      role="menu"
      :aria-label="label"
      class="absolute z-40 mt-2 min-w-56 rounded-panel border border-line bg-surface p-1.5 text-ink shadow-panel"
      :class="align === 'start' ? 'left-0' : 'right-0'"
    >
      <slot />
    </div>
  </div>
</template>
