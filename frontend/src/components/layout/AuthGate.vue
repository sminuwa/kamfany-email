<script setup lang="ts">
import { RouterLink } from "vue-router";
import BrandMark from "../common/BrandMark.vue";
import { useBrandStore } from "../../stores/brand";
import ThemeMenu from "./ThemeMenu.vue";

const brand = useBrandStore();
</script>

<template>
  <div class="relative min-h-dvh bg-canvas text-ink">
    <div class="absolute right-5 top-5 z-10">
      <ThemeMenu />
    </div>
    <div class="grid min-h-dvh place-items-center px-5 py-20">
      <div class="auth-gate w-full max-w-[24.5rem]">
        <RouterLink to="/" class="mb-8 flex items-center justify-center gap-3 text-ink">
          <BrandMark :layout="brand.logoUrl ? 'lockup' : 'mark'" :class="brand.logoUrl ? 'h-12' : 'size-11'" />
          <span v-if="brand.showName" class="text-[1.35rem] font-semibold tracking-[-0.03em]">{{ brand.name }}</span>
        </RouterLink>
        <div class="km-card px-6 py-7 sm:px-8 sm:py-8">
          <slot />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.auth-gate {
  animation: auth-settle 480ms cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes auth-settle {
  from {
    transform: translateY(10px);
  }
  to {
    transform: translateY(0);
  }
}
</style>
