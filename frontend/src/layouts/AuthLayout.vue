<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";
import BrandMark from "../components/common/BrandMark.vue";
import ThemeMenu from "../components/layout/ThemeMenu.vue";
</script>

<template>
  <div class="relative min-h-dvh bg-canvas text-ink">
    <div class="absolute right-4 top-4">
      <ThemeMenu />
    </div>
    <div class="mx-auto flex min-h-dvh w-full max-w-3xl flex-col px-6 py-16">
      <RouterLink to="/" class="mb-10 flex items-center gap-3 text-ink">
        <BrandMark class="size-10" />
        <span class="text-xl font-semibold tracking-[-0.03em]">KAMFANY Mail</span>
      </RouterLink>
      <RouterView />
    </div>
  </div>
</template>
