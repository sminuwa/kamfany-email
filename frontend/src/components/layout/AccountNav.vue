<script setup lang="ts">
import { Lock, Mail, Monitor, ScrollText, Server, User, Users } from "@lucide/vue";
import { computed } from "vue";
import { RouterLink, useRoute } from "vue-router";
import type { Component } from "vue";
import { useAuthStore } from "../../stores/auth";

const auth = useAuthStore();
const route = useRoute();

const links = computed(() => {
  const account: Array<{ to: string; name: string; label: string; icon: Component }> = [
    { to: "/account", name: "account", label: "Account settings", icon: User },
    { to: "/account/password", name: "account-password", label: "Change password", icon: Lock },
    { to: "/account/sessions", name: "account-sessions", label: "Sessions", icon: Monitor },
  ];
  const desk: Array<{ to: string; name: string; label: string; icon: Component }> = [
    { to: "/account/mailboxes", name: "account-mailboxes", label: "Mailboxes", icon: Mail },
  ];
  if (auth.canManageMailboxes) {
    desk.push({ to: "/account/provider", name: "account-provider", label: "Email provider", icon: Server });
  }
  if (auth.canReadAudit) {
    desk.push({ to: "/account/audit", name: "account-audit", label: "Audit log", icon: ScrollText });
  }
  if (auth.canManageUsers) {
    desk.push({ to: "/account/people", name: "account-people", label: "People", icon: Users });
  }
  return [
    { label: "Account", items: account },
    { label: "Desk", items: desk },
  ];
});
</script>

<template>
  <nav aria-label="Account" class="flex flex-col gap-5">
    <div v-for="group in links" :key="group.label">
      <p class="px-3 pb-1.5 text-xs font-semibold text-muted">{{ group.label }}</p>
      <div class="flex flex-col gap-0.5">
        <RouterLink
          v-for="link in group.items"
          :key="link.name"
          :to="link.to"
          class="flex h-10 items-center gap-3 rounded-full px-3 text-sm"
          :class="route.name === link.name ? 'account-current font-semibold' : 'text-muted hover:bg-canvas hover:text-ink'"
          :aria-current="route.name === link.name ? 'page' : undefined"
        >
          <component :is="link.icon" class="size-4 shrink-0" />
          {{ link.label }}
        </RouterLink>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.account-current {
  color: var(--km-nav-current);
  background: var(--km-nav-current-bg);
}
</style>
