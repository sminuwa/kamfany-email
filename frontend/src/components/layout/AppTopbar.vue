<script setup lang="ts">
import {
  Bell,
  CircleHelp,
  Menu,
  PanelLeft,
  RefreshCw,
  Search,
  SlidersHorizontal,
} from "@lucide/vue";
import { RouterLink, useRouter } from "vue-router";
import { computed, ref } from "vue";
import { useRailCollapsed } from "../../composables/useRail";
import { useAuthStore } from "../../stores/auth";
import { useMailboxStore } from "../../stores/mailboxes";
import { useMailStore } from "../../stores/messages";
import { useNoticeStore } from "../../stores/notices";
import type { DeskNotice } from "../../services/notice.service";
import { useServiceStore, type ServiceStatus } from "../../stores/service";
import { useUiStore } from "../../stores/ui";
import MenuPanel from "../common/MenuPanel.vue";
import ThemeMenu from "./ThemeMenu.vue";

const router = useRouter();
const ui = useUiStore();
const mail = useMailStore();
const auth = useAuthStore();
const boxes = useMailboxStore();
const notices = useNoticeStore();
const service = useServiceStore();
const { isDesktop } = useRailCollapsed();
const notificationsOpen = ref(false);
const helpOpen = ref(false);
const profileOpen = ref(false);

const statusClass: Record<ServiceStatus, string> = {
  unknown: "bg-muted",
  ready: "bg-status-ready",
  degraded: "bg-status-degraded",
  offline: "bg-status-offline",
};

async function leaveDesk() {
  profileOpen.value = false;
  await auth.logout();
  await router.push("/account");
}

function toggleNavigation() {
  if (isDesktop.value) {
    ui.toggleCollapsed();
  } else {
    ui.toggleMobile();
  }
}

const filtersActive = computed(
  () =>
    mail.filters.unread ||
    mail.filters.attachments ||
    mail.filters.important ||
    mail.search.from.trim() !== "" ||
    mail.search.to.trim() !== "" ||
    mail.search.subject.trim() !== "" ||
    mail.search.after !== "" ||
    mail.search.before !== "" ||
    mail.search.folder !== "" ||
    mail.search.labelId !== "" ||
    mail.search.allMailboxes,
);

function clearSearch() {
  ui.searchQuery = "";
  mail.clearFilters();
}

async function openNotice(item: DeskNotice) {
  notificationsOpen.value = false;
  await notices.mark(item.id);
  if (item.mailboxId) {
    boxes.select(item.mailboxId);
  }
  if (item.messageId) {
    await router.push({ path: "/", query: { m: item.messageId } });
  }
}

function navigationLabel() {
  if (!isDesktop.value) {
    return ui.mobileOpen ? "Close menu" : "Open menu";
  }
  return ui.collapsed ? "Expand sidebar" : "Collapse sidebar";
}
</script>

<template>
  <header class="desk-chrome flex h-16 shrink-0 items-center gap-1.5 px-3 min-[960px]:gap-2">
    <button
      type="button"
      class="grid size-9 shrink-0 place-items-center rounded-control text-ink hover:bg-surface min-[960px]:size-10"
      :aria-expanded="isDesktop ? !ui.collapsed : ui.mobileOpen"
      :aria-label="navigationLabel()"
      @click="toggleNavigation"
    >
      <Menu v-if="!isDesktop" class="size-4" />
      <PanelLeft v-else class="size-4" />
    </button>

    <form class="relative min-w-0 flex-1" role="search" @submit.prevent>
      <label class="sr-only" for="mail-search">Search mail</label>
      <Search class="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 text-muted" />
      <input
        id="mail-search"
        v-model="ui.searchQuery"
        type="search"
        :placeholder="isDesktop ? 'Search mail' : 'Search'"
        autocomplete="off"
        class="h-11 w-full rounded-full border border-line bg-surface pl-11 pr-12 text-base text-ink shadow-panel sm:text-sm"
      />
      <div class="absolute right-1 top-1">
      <MenuPanel v-model="ui.filtersOpen" label="Search filters">
        <template #trigger="{ toggle, open }">
          <button
            type="button"
            class="relative grid size-8 place-items-center rounded-full text-muted hover:bg-canvas hover:text-ink"
            :aria-expanded="open"
            aria-haspopup="menu"
            aria-label="Search filters"
            @click="toggle"
          >
            <SlidersHorizontal class="size-4" />
            <span
              v-if="filtersActive"
              class="absolute right-1 top-1 size-1.5 rounded-full bg-brass"
            />
          </button>
        </template>
        <div class="w-72">
          <p class="px-2.5 py-1.5 text-xs font-semibold text-muted">Show</p>
          <label class="flex items-center gap-2 rounded-[8px] px-2.5 py-2 text-sm hover:bg-canvas">
            <input v-model="mail.filters.unread" type="checkbox" class="size-4 accent-brass" />
            Unread
          </label>
          <label class="flex items-center gap-2 rounded-[8px] px-2.5 py-2 text-sm hover:bg-canvas">
            <input v-model="mail.filters.attachments" type="checkbox" class="size-4 accent-brass" />
            Has attachment
          </label>
          <label class="flex items-center gap-2 rounded-[8px] px-2.5 py-2 text-sm hover:bg-canvas">
            <input v-model="mail.filters.important" type="checkbox" class="size-4 accent-brass" />
            Important
          </label>
          <label class="mt-1 block px-2.5 text-sm">
            From
            <input v-model="mail.search.from" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm" />
          </label>
          <label class="mt-2 block px-2.5 text-sm">
            To
            <input v-model="mail.search.to" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm" />
          </label>
          <label class="mt-2 block px-2.5 text-sm">
            Subject
            <input v-model="mail.search.subject" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm" />
          </label>
          <div class="mt-2 grid grid-cols-2 gap-2 px-2.5">
            <label class="text-sm">
              After
              <input v-model="mail.search.after" type="date" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm" />
            </label>
            <label class="text-sm">
              Before
              <input v-model="mail.search.before" type="date" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm" />
            </label>
          </div>
          <label class="mt-2 block px-2.5 text-sm">
            Folder
            <select v-model="mail.search.folder" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm">
              <option value="">Any folder</option>
              <option value="inbox">Inbox</option>
              <option value="sent">Sent</option>
              <option value="archive">Archive</option>
              <option value="spam">Spam</option>
              <option value="trash">Trash</option>
              <option value="starred">Starred</option>
              <option value="snoozed">Snoozed</option>
              <option v-for="folder in mail.folders.filter((item) => !item.systemKey)" :key="folder.id" :value="folder.id">
                {{ folder.name }}
              </option>
            </select>
          </label>
          <label class="mt-2 block px-2.5 text-sm">
            Label
            <select v-model="mail.search.labelId" class="mt-1 h-9 w-full rounded-control border border-line bg-canvas px-2 text-sm">
              <option value="">Any label</option>
              <option v-for="label in mail.labels" :key="label.id" :value="label.id">{{ label.name }}</option>
            </select>
          </label>
          <label class="mt-2 flex items-center gap-2 px-2.5 py-2 text-sm">
            <input v-model="mail.search.allMailboxes" type="checkbox" class="size-4 accent-brass" />
            Every mailbox you can read
          </label>
          <button
            type="button"
            class="mt-1 w-full rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
            @click="clearSearch()"
          >
            Clear filters
          </button>
        </div>
      </MenuPanel>
      </div>
    </form>

    <div class="flex shrink-0 items-center">
      <span
        class="mr-1 size-2 rounded-full"
        :class="statusClass[service.status]"
        role="status"
        :aria-label="service.label"
      />
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-surface min-[960px]:size-10"
        :aria-busy="service.checking"
        aria-label="Refresh"
        @click="service.refresh()"
      >
        <RefreshCw class="size-4" :class="service.checking ? 'animate-spin' : ''" />
      </button>

      <MenuPanel v-model="notificationsOpen" label="Notifications">
        <template #trigger="{ toggle, open }">
          <button
            type="button"
            class="relative grid size-9 place-items-center rounded-control text-ink hover:bg-surface min-[960px]:size-10"
            :aria-expanded="open"
            aria-haspopup="menu"
            aria-label="Notifications"
            @click="toggle"
          >
            <Bell class="size-4" />
            <span
              v-if="notices.unread"
              class="absolute right-1.5 top-1.5 size-1.5 rounded-full bg-brass"
            />
          </button>
        </template>
        <div class="w-72">
          <div class="flex items-center justify-between px-2.5 py-1.5">
            <p class="text-sm font-semibold">Notifications</p>
            <button v-if="notices.unread" type="button" class="text-xs font-medium text-muted hover:text-ink" @click="notices.markAll()">
              Mark read
            </button>
          </div>
          <p v-if="notices.items.length === 0" class="px-2.5 py-2 text-sm text-muted">No notifications.</p>
          <ul v-else class="max-h-80 overflow-y-auto">
            <li v-for="item in notices.items" :key="item.id">
              <button
                type="button"
                class="block w-full rounded-[8px] px-2.5 py-2 text-left hover:bg-canvas"
                @click="openNotice(item)"
              >
                <span class="block truncate text-sm" :class="item.readAt ? 'font-medium' : 'font-semibold'">{{ item.title }}</span>
                <span class="block truncate text-sm text-muted">{{ item.body }}</span>
              </button>
            </li>
          </ul>
        </div>
      </MenuPanel>

      <ThemeMenu />

      <MenuPanel v-model="helpOpen" label="Help" class="hidden sm:block">
        <template #trigger="{ toggle, open }">
          <button
            type="button"
            class="grid size-9 place-items-center rounded-control text-ink hover:bg-surface min-[960px]:size-10"
            :aria-expanded="open"
            aria-haspopup="menu"
            aria-label="Help"
            @click="toggle"
          >
            <CircleHelp class="size-4" />
          </button>
        </template>
        <p class="max-w-64 px-2.5 py-2 text-sm leading-relaxed">
          Search with Ctrl K or Command K. Esc closes an open menu.
        </p>
        <p class="max-w-64 px-2.5 pb-2 text-sm leading-relaxed text-muted">
          KAMFANY Mail for IT KAMFANY LIMITED.
        </p>
      </MenuPanel>

      <MenuPanel v-model="profileOpen" label="Account">
        <template #trigger="{ toggle, open }">
          <button
            type="button"
            class="grid size-9 place-items-center rounded-control hover:bg-surface min-[960px]:size-10"
            :aria-expanded="open"
            aria-haspopup="menu"
            aria-label="Account menu"
            @click="toggle"
          >
            <span class="grid size-8 place-items-center rounded-full bg-lagoon text-xs font-semibold text-brass">
              {{ auth.user ? auth.user.displayName.slice(0, 1) : "K" }}
            </span>
          </button>
        </template>
        <p class="px-2.5 py-2 text-sm font-semibold">
          {{ auth.user?.displayName ?? "Not signed in" }}
        </p>
        <p v-if="auth.user" class="px-2.5 pb-2 text-sm text-muted">{{ auth.user.email }}</p>
        <p class="px-2.5 pb-2 text-sm leading-relaxed text-muted sm:hidden">
          Search with Ctrl K or Command K.
        </p>
        <RouterLink
          to="/account"
          role="menuitem"
          class="block rounded-[8px] px-2.5 py-2 text-sm hover:bg-canvas"
          @click="profileOpen = false"
        >
          Account settings
        </RouterLink>
        <button
          v-if="auth.user"
          type="button"
          role="menuitem"
          class="block w-full rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
          @click="leaveDesk"
        >
          Sign out
        </button>
      </MenuPanel>
    </div>
  </header>
</template>
