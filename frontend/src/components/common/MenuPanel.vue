<script setup lang="ts">
import { onClickOutside, onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";

const open = defineModel<boolean>({ required: true });

withDefaults(
  defineProps<{
    align?: "start" | "end";
    side?: "down" | "up";
    label: string;
  }>(),
  { align: "end", side: "down" },
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
      class="km-card absolute z-40 min-w-56 p-1.5 text-ink"
      :class="[
        align === 'start' ? 'left-0' : 'right-0',
        side === 'up' ? 'bottom-full mb-2' : 'top-full mt-2',
      ]"
    >
      <slot />
    </div>
  </div>
</template>
