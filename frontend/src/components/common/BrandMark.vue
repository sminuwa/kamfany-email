<script setup lang="ts">
import { ref } from "vue";
import { useBrandStore } from "../../stores/brand";

withDefaults(
  defineProps<{
    layout?: "mark" | "lockup";
  }>(),
  { layout: "mark" },
);

const brand = useBrandStore();
const failed = ref(false);

function onLoad(event: Event) {
  const image = event.target as HTMLImageElement;
  const height = image.naturalHeight || 1;
  brand.noteLogo(image.naturalWidth / height > 1.4);
}

function onError() {
  failed.value = true;
  brand.noteLogo(false);
}
</script>

<template>
  <span class="inline-flex max-w-full items-center justify-center">
    <img
      v-if="brand.logoUrl && !failed"
      :src="brand.logoUrl"
      :alt="brand.name"
      class="brand-logo block h-full"
      :class="layout === 'lockup' ? 'w-auto max-w-full object-contain' : 'w-full object-cover object-left'"
      @load="onLoad"
      @error="onError"
    />
    <svg v-else viewBox="0 0 32 32" aria-hidden="true" class="block aspect-square h-full">
      <rect width="32" height="32" rx="8" fill="#F2C14E" />
      <path
        d="M8 12.2h16v10.2H8z"
        fill="none"
        stroke="#1A1408"
        stroke-width="2.2"
        stroke-linejoin="round"
      />
      <path
        d="M8.4 12.6 16 18.4 23.6 12.6"
        fill="none"
        stroke="#1A1408"
        stroke-width="2.2"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </span>
</template>

<style scoped>
:global([data-theme="dark"]) .brand-logo {
  filter: invert(1) hue-rotate(180deg);
}
</style>
