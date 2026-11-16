<script setup lang="ts">
import { Lock, Mail, Monitor, ScrollText, Server, User } from "@lucide/vue";
import { computed } from "vue";
import { RouterLink, useRoute } from "vue-router";
import type { Component } from "vue";
import { useAuthStore } from "../../stores/auth";

const auth = useAuthStore();
const route = useRoute();

const links = computed(() => {
  const items: Array<{ to: string; name: string; label: string; icon: Component }> = [
    { to: "/account", name: "account", label: "Account settings", icon: User },
    { to: "/account/password", name: "account-password", label: "Change password", icon: Lock },
    { to: "/account/sessions", name: "account-sessions", label: "Sessions", icon: Monitor },
  ];
  if (auth.canManageMailboxes) {
    items.push({ to: "/account/provider", name: "account-provider", label: "Email provider", icon: Server });
  }
  if (auth.canReadAudit) {
    items.push({ to: "/account/audit", name: "account-audit", label: "Audit log", icon: ScrollText });
  }
  if (auth.canManageUsers) {
    items.push({ to: "/account/mailboxes", name: "account-mailboxes", label: "Mailboxes", icon: Mail });
  }
  return items;
});
</script>

<template>
  <nav aria-label="Account" class="flex flex-col">
    <RouterLink
      v-for="link in links"
      :key="link.name"
      :to="link.to"
      role="menuitem"
      class="flex h-9 items-center gap-2 rounded-[8px] px-2.5 text-sm"
      :class="route.name === link.name ? 'bg-canvas font-semibold text-ink' : 'text-ink hover:bg-canvas'"
      :aria-current="route.name === link.name ? 'page' : undefined"
    >
      <component :is="link.icon" class="size-4 shrink-0 text-muted" />
      {{ link.label }}
    </RouterLink>
  </nav>
</template>
