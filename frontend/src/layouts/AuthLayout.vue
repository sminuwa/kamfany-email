<script setup lang="ts">
import { Menu } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { RouterLink, RouterView, useRoute } from "vue-router";
import BrandMark from "../components/common/BrandMark.vue";
import AccountNav from "../components/layout/AccountNav.vue";
import ThemeMenu from "../components/layout/ThemeMenu.vue";
import { useAuthStore } from "../stores/auth";

const route = useRoute();
const auth = useAuthStore();
const navOpen = ref(false);
const centered = computed(() => route.name === "reset-password" || auth.status !== "signed_in");
const initial = computed(() => auth.user?.displayName.slice(0, 1) || "K");

watch(
  () => route.path,
  () => {
    navOpen.value = false;
  },
);
</script>

<template>
  <div v-if="centered" class="relative min-h-dvh bg-canvas text-ink">
    <div class="absolute right-5 top-5 z-10">
      <ThemeMenu />
    </div>
    <div class="grid min-h-dvh place-items-center px-5 py-20">
      <div class="auth-gate w-full max-w-[24.5rem]">
        <RouterLink to="/" class="mb-8 flex items-center justify-center gap-3 text-ink">
          <BrandMark class="size-11" />
          <span class="text-[1.35rem] font-semibold tracking-[-0.03em]">KAMFANY Mail</span>
        </RouterLink>
        <div class="rounded-panel border border-line bg-surface px-6 py-7 shadow-panel sm:px-8 sm:py-8">
          <RouterView />
        </div>
      </div>
    </div>
  </div>

  <div v-else class="flex h-dvh min-h-0 bg-canvas text-ink">
    <button
      v-if="navOpen"
      type="button"
      class="fixed inset-0 z-20 bg-ink/40 min-[960px]:hidden"
      aria-label="Close menu"
      @click="navOpen = false"
    />
    <aside
      class="fixed inset-y-0 left-0 z-30 flex w-[248px] flex-col border-r border-line bg-surface transition-transform duration-200 ease-out min-[960px]:translate-x-0"
      :class="navOpen ? 'translate-x-0' : '-translate-x-full'"
    >
      <RouterLink to="/" class="flex items-center gap-3 px-5 pb-2 pt-5 text-ink">
        <BrandMark class="size-9" />
        <span class="min-w-0">
          <span class="block truncate text-[15px] font-semibold tracking-[-0.03em]">KAMFANY Mail</span>
        </span>
      </RouterLink>
      <div class="min-h-0 flex-1 overflow-y-auto px-3 pb-4 pt-2">
        <AccountNav />
      </div>
      <div class="border-t border-line p-3">
        <RouterLink
          to="/account"
          class="flex items-center gap-3 rounded-full px-2 py-2 hover:bg-canvas"
          :aria-current="route.name === 'account' ? 'page' : undefined"
        >
          <span class="grid size-9 shrink-0 place-items-center rounded-full bg-lagoon text-xs font-semibold text-brass">
            {{ initial }}
          </span>
          <span class="min-w-0">
            <span class="block truncate text-sm font-semibold">{{ auth.user?.displayName }}</span>
            <span class="block truncate text-xs text-muted">{{ auth.user?.email }}</span>
          </span>
        </RouterLink>
      </div>
    </aside>

    <div class="flex min-w-0 flex-1 flex-col min-[960px]:pl-[248px]">
      <header class="flex h-14 shrink-0 items-center gap-2 border-b border-line bg-surface px-4">
        <button
          type="button"
          class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas min-[960px]:hidden"
          aria-label="Open menu"
          @click="navOpen = true"
        >
          <Menu class="size-4" />
        </button>
        <RouterLink to="/" class="text-sm text-muted hover:text-ink">Open the desk</RouterLink>
        <div class="ml-auto flex items-center gap-1">
          <ThemeMenu />
          <span class="grid size-8 place-items-center rounded-full bg-lagoon text-xs font-semibold text-brass">
            {{ initial }}
          </span>
        </div>
      </header>
      <div class="account-stage relative min-h-0 flex-1 overflow-hidden">
        <div class="account-art" aria-hidden="true">
          <svg class="account-scene" viewBox="0 0 480 118">
            <path class="scene-ribbon" d="M8 28c70 16 124-26 206-6 74 18 132-12 198 4" />
            <path class="scene-ribbon-soft" d="M36 8c58 16 110-18 184 0 58 14 108-8 160 6" />
            <rect class="scene-square" x="132" y="22" width="15" height="15" rx="3.5" transform="rotate(-16 139 29)" />
            <rect class="scene-square-deep" x="156" y="8" width="11" height="11" rx="2.5" transform="rotate(14 161 13)" />
            <rect class="scene-square" x="116" y="44" width="8" height="8" rx="2" transform="rotate(8 120 48)" />
            <g transform="translate(186 6) rotate(11)">
              <path
                class="scene-body"
                d="M0 30h150a12 12 0 0 1 12 12v50a12 12 0 0 1-12 12H12a12 12 0 0 1-12-12V42a12 12 0 0 1 12-12Z"
              />
              <path class="scene-flap" d="M14 42 81 70l67-28-67 16Z" />
              <g transform="translate(30 -2) rotate(-13)">
                <rect class="scene-letter-shadow" x="1.5" y="2.5" width="96" height="62" rx="8" />
                <rect class="scene-letter" width="96" height="62" rx="8" />
                <path class="scene-lines" d="M14 18h54M14 30h40M14 42h26" />
              </g>
              <path class="scene-pocket" d="M0 62 81 94l81-32v22a12 12 0 0 1-12 12H12A12 12 0 0 1 0 84Z" />
            </g>
          </svg>
        </div>
        <div class="relative z-10 h-full overflow-y-auto px-4 py-6 min-[960px]:px-8">
          <RouterView />
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

.account-stage {
  --ribbon: var(--km-art-ribbon);
  --ribbon-soft: var(--km-art-ribbon-soft);
  --square: var(--km-art-square);
  --square-deep: var(--km-art-square-deep);
  --letter: var(--km-art-letter);
  --letter-edge: var(--km-art-letter-edge);
  --letter-line: var(--km-art-letter-line);
  --letter-shadow: var(--km-art-letter-shadow);
  --flap: var(--km-art-flap);
  --body: var(--km-art-body);
  --body-edge: var(--km-art-body-edge);
  --pocket: var(--km-art-pocket);
  background: var(--km-account-wash);
}

.account-art {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}

.account-scene {
  position: absolute;
  top: 0;
  right: 210px;
  width: 460px;
  height: auto;
}

.scene-ribbon,
.scene-ribbon-soft {
  fill: none;
  stroke-linecap: round;
}

.scene-ribbon {
  stroke: var(--ribbon);
  stroke-width: 22;
}

.scene-ribbon-soft {
  stroke: var(--ribbon-soft);
  stroke-width: 16;
}

.scene-square {
  fill: var(--square);
}

.scene-square-deep {
  fill: var(--square-deep);
}

.scene-letter,
.scene-letter-shadow {
  fill: var(--letter);
}

.scene-letter {
  stroke: var(--letter-edge);
  stroke-width: 1;
}

.scene-letter-shadow {
  fill: var(--letter-shadow);
}

.scene-lines {
  fill: none;
  stroke: var(--letter-line);
  stroke-width: 3.5;
  stroke-linecap: round;
}

.scene-flap {
  fill: var(--flap);
}

.scene-body {
  fill: var(--body);
  stroke: var(--body-edge);
  stroke-width: 1.5;
}

.scene-pocket {
  fill: var(--pocket);
}

@media (max-width: 900px) {
  .account-scene {
    right: -36px;
    width: 280px;
    opacity: 0.7;
  }
}
</style>
