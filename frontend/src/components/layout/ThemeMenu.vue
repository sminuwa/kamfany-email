<script setup lang="ts">
import { Monitor, Moon, Sun } from "@lucide/vue";
import { ref } from "vue";
import { usePreferencesStore } from "../../stores/preferences";
import type { ThemePreference } from "../../utils/theme";
import MenuPanel from "../common/MenuPanel.vue";

const preferences = usePreferencesStore();
const open = ref(false);

const options: Array<{ id: ThemePreference; label: string; icon: typeof Sun }> = [
  { id: "light", label: "Light", icon: Sun },
  { id: "dark", label: "Dark", icon: Moon },
  { id: "system", label: "System", icon: Monitor },
];

function choose(theme: ThemePreference) {
  preferences.setTheme(theme);
  open.value = false;
}
</script>

<template>
  <MenuPanel v-model="open" label="Theme">
    <template #trigger="{ toggle, open: isOpen }">
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-ink/5 min-[960px]:size-10"
        :aria-expanded="isOpen"
        aria-haspopup="menu"
        aria-label="Theme"
        @click="toggle"
      >
        <Sun v-if="preferences.preference === 'light'" class="size-4" />
        <Moon v-else-if="preferences.preference === 'dark'" class="size-4" />
        <Monitor v-else class="size-4" />
      </button>
    </template>
    <button
      v-for="option in options"
      :key="option.id"
      type="button"
      role="menuitemradio"
      :aria-checked="preferences.preference === option.id"
      class="flex w-full items-center gap-2 rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
      :class="preferences.preference === option.id ? 'font-semibold' : ''"
      @click="choose(option.id)"
    >
      <component :is="option.icon" class="size-4 text-muted" />
      {{ option.label }}
    </button>
  </MenuPanel>
</template>
