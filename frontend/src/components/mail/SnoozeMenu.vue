<script setup lang="ts">
import { AlarmClock } from "@lucide/vue";
import { computed, ref } from "vue";
import { snoozeChoices, snoozeInstant } from "../../utils/snooze";
import MenuPanel from "../common/MenuPanel.vue";

withDefaults(
  defineProps<{
    compact?: boolean;
    unsnooze?: boolean;
  }>(),
  { compact: false, unsnooze: false },
);

const emit = defineEmits<{
  snooze: [until: string | null];
}>();

const open = ref(false);
const when = ref("");
const note = ref("");
const choices = computed(() => (open.value ? snoozeChoices() : []));

function choose(until: string | null) {
  emit("snooze", until);
  open.value = false;
  when.value = "";
  note.value = "";
}

function pick() {
  const until = snoozeInstant(when.value);
  if (!until) {
    note.value = "Choose a future time within a year.";
    return;
  }
  choose(until);
}
</script>

<template>
  <MenuPanel v-model="open" label="Snooze" :align="compact ? 'end' : 'start'">
    <template #trigger="{ toggle, open: isOpen }">
      <button
        type="button"
        class="rounded-control text-sm text-ink hover:bg-canvas"
        :class="compact ? 'grid size-9 place-items-center' : 'inline-flex h-8 items-center gap-1.5 px-2'"
        :aria-expanded="isOpen"
        aria-haspopup="menu"
        aria-label="Snooze"
        @click="toggle"
      >
        <AlarmClock class="size-4" />
        <span v-if="!compact">Snooze</span>
      </button>
    </template>
    <button
      v-for="choice in choices"
      :key="choice.id"
      type="button"
      role="menuitem"
      class="flex w-full items-center rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      @click="choose(choice.until)"
    >
      {{ choice.label }}
    </button>
    <form class="space-y-1.5 px-2.5 py-2" @submit.prevent="pick">
      <input
        v-model="when"
        type="datetime-local"
        required
        class="h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm"
        aria-label="Wake time"
      />
      <button type="submit" class="h-8 rounded-control px-2 text-sm hover:bg-canvas">Set time</button>
      <p v-if="note" class="text-xs text-muted">{{ note }}</p>
    </form>
    <button
      v-if="unsnooze"
      type="button"
      role="menuitem"
      class="flex w-full items-center rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      @click="choose(null)"
    >
      Unsnooze
    </button>
  </MenuPanel>
</template>
