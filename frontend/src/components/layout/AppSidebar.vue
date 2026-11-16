<script setup lang="ts">
import {
  AlarmClock,
  Archive,
  BookUser,
  ChevronDown,
  FilePen,
  Inbox,
  Mailbox,
  PenLine,
  Send,
  ShieldAlert,
  Star,
  Trash2,
} from "@lucide/vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import type { Component } from "vue";
import { computed, ref } from "vue";
import { useRailCollapsed } from "../../composables/useRail";
import { useMailboxStore } from "../../stores/mailboxes";
import { useMailStore } from "../../stores/messages";
import { useUiStore } from "../../stores/ui";
import type { LabelTone } from "../../types/mail";
import { systemFolders } from "../../utils/folders";
import BrandMark from "../common/BrandMark.vue";
import MenuPanel from "../common/MenuPanel.vue";

const ui = useUiStore();
const mail = useMailStore();
const boxes = useMailboxStore();
const route = useRoute();
const router = useRouter();
const { railCollapsed } = useRailCollapsed();
const mailboxOpen = ref(false);
const addingFolder = ref(false);
const addingLabel = ref(false);
const folderName = ref("");
const labelName = ref("");
const tone = ref<LabelTone>("brass");
const mailboxLabel = computed(() => boxes.active?.address || "No mailbox");
const customFolders = computed(() => mail.folders.filter((folder) => !folder.systemKey));

const railTone: Record<LabelTone, string> = {
  brass: "bg-brass",
  paper: "bg-lagoon-ink",
  mist: "bg-lagoon-muted",
};

const unreadCounts = computed(() => {
  const counts: Record<string, number> = {};
  for (const folder of systemFolders) {
    counts[folder.key] = mail.unreadFor(folder.key);
  }
  return counts;
});

async function addFolder() {
  const name = folderName.value.trim();
  if (!name) {
    return;
  }
  await mail.createNamedFolder(name, tone.value);
  folderName.value = "";
  addingFolder.value = false;
}

async function addLabel() {
  const name = labelName.value.trim();
  if (!name) {
    return;
  }
  await mail.createNamedLabel(name, tone.value);
  labelName.value = "";
  addingLabel.value = false;
}

function chooseFolder() {
  mail.clearLabelFilter();
  ui.closeMobile();
}

function chooseLabel(id: string) {
  mail.toggleLabelFilter(id);
  ui.closeMobile();
}

function switchMailbox(id: string) {
  boxes.select(id);
  mailboxOpen.value = false;
  ui.closeMobile();
  const query = { ...route.query };
  delete query.m;
  void router.replace({ path: route.path, query });
}

const icons: Record<string, Component> = {
  inbox: Inbox,
  starred: Star,
  snoozed: AlarmClock,
  sent: Send,
  drafts: FilePen,
  archive: Archive,
  spam: ShieldAlert,
  trash: Trash2,
};
</script>

<template>
  <aside
    class="desk-chrome fixed inset-y-0 left-0 z-30 flex w-[248px] flex-col bg-lagoon text-lagoon-ink transition-transform duration-200 ease-out min-[960px]:static min-[960px]:z-20 min-[960px]:w-auto min-[960px]:translate-x-0"
    :class="ui.mobileOpen ? 'translate-x-0' : '-translate-x-full'"
  >
    <div class="px-3 pb-2 pt-4">
      <div class="flex items-center gap-3 px-1" :class="railCollapsed ? 'justify-center' : ''">
        <BrandMark class="size-9 shrink-0" />
        <p
          v-show="!railCollapsed"
          class="truncate text-[15px] font-semibold tracking-[-0.02em]"
        >
          KAMFANY Mail
        </p>
      </div>

      <MenuPanel v-model="mailboxOpen" align="start" label="Mailboxes" class="mt-4">
        <template #trigger="{ toggle, open }">
          <button
            type="button"
            class="flex h-10 w-full items-center gap-2 rounded-control border border-lagoon-line px-2.5 text-left text-sm hover:bg-lagoon-active"
            :class="railCollapsed ? 'justify-center px-0' : ''"
            :aria-expanded="open"
            aria-haspopup="menu"
            :aria-label="railCollapsed ? 'Mailboxes' : undefined"
            :title="railCollapsed ? 'Mailboxes' : undefined"
            @click="toggle"
          >
            <Mailbox class="size-4 shrink-0 text-lagoon-muted" />
            <span v-show="!railCollapsed" class="min-w-0 flex-1 truncate">{{ mailboxLabel }}</span>
            <ChevronDown v-show="!railCollapsed" class="size-4 shrink-0 text-lagoon-muted" />
          </button>
        </template>
        <p class="px-2.5 py-2 text-sm font-semibold text-ink">{{ mailboxLabel }}</p>
        <p v-if="boxes.readable.length === 0" class="px-2.5 pb-2 text-sm leading-relaxed text-muted">
          No mailbox is available to this account.
        </p>
        <ul v-else class="max-h-64 overflow-y-auto pb-1">
          <li v-for="mailbox in boxes.readable" :key="mailbox.id">
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 rounded-[8px] px-2.5 py-2 text-left text-sm text-ink hover:bg-canvas"
              @click="switchMailbox(mailbox.id)"
            >
              <span class="min-w-0 flex-1 truncate">{{ mailbox.address }}</span>
              <span v-if="mailbox.id === boxes.active?.id" class="text-xs text-muted">Current</span>
            </button>
          </li>
        </ul>
        <p v-if="boxes.active && !boxes.active.isReceivingVerified" class="px-2.5 pb-2 text-sm leading-relaxed text-muted">
          Receiving is not verified yet.
        </p>
      </MenuPanel>

      <button
        type="button"
        class="mt-3 flex h-10 items-center justify-center gap-2 rounded-full bg-lagoon-ink text-sm font-semibold text-[#10211f] transition-colors duration-200 ease-out hover:bg-brass hover:text-brass-ink"
        :class="railCollapsed ? 'mx-auto w-10 px-0' : 'w-full px-4'"
        :aria-label="railCollapsed ? 'Compose' : undefined"
        :title="railCollapsed ? 'Compose' : undefined"
        @click="ui.openComposer()"
      >
        <PenLine class="size-4" />
        <span v-show="!railCollapsed">Compose</span>
      </button>
    </div>

    <nav class="mt-3 min-h-0 flex-1 overflow-y-auto px-3 pb-4" aria-label="Folders">
      <ul class="space-y-0.5">
        <li v-for="folder in systemFolders" :key="folder.key">
          <RouterLink
            :to="folder.path"
            class="flex h-9 items-center gap-3 rounded-control text-sm text-lagoon-ink hover:bg-lagoon-active"
            :class="[
              route.path === folder.path ? 'bg-lagoon-active' : '',
              railCollapsed ? 'justify-center px-0' : 'px-2.5',
            ]"
            :aria-current="route.path === folder.path ? 'page' : undefined"
            :title="railCollapsed ? folder.label : undefined"
            @click="chooseFolder"
          >
            <span class="relative">
              <component
                :is="icons[folder.key]"
                class="size-4 shrink-0"
                :class="route.path === folder.path ? 'text-brass' : 'text-lagoon-muted'"
              />
              <span
                v-if="railCollapsed && unreadCounts[folder.key]"
                class="absolute -right-1.5 -top-1.5 size-1.5 rounded-full bg-brass"
              />
            </span>
            <span v-show="!railCollapsed" class="truncate">{{ folder.label }}</span>
            <span
              v-if="!railCollapsed && unreadCounts[folder.key]"
              class="ml-auto rounded-full bg-brass px-1.5 text-[11px] font-semibold tabular-nums text-brass-ink"
            >
              {{ unreadCounts[folder.key] }}
            </span>
          </RouterLink>
        </li>
      </ul>

      <div v-if="!railCollapsed" class="mt-6">
        <p class="px-2.5 text-xs font-semibold text-lagoon-muted">Folders</p>
        <p v-if="customFolders.length === 0" class="px-2.5 pt-2 text-sm text-lagoon-muted">None yet</p>
        <ul v-else class="mt-1 space-y-0.5">
          <li v-for="folder in customFolders" :key="folder.id">
            <RouterLink
              :to="`/folders/${folder.id}`"
              class="flex h-8 items-center gap-2 rounded-control px-2.5 text-sm text-lagoon-ink hover:bg-lagoon-active"
              :class="route.params.folderId === folder.id ? 'bg-lagoon-active' : ''"
              @click="chooseFolder"
            >
              <span class="size-2 shrink-0 rounded-full" :class="railTone[folder.color ?? 'paper']" />
              <span class="truncate">{{ folder.name }}</span>
              <span v-if="folder.unread" class="ml-auto text-[11px] font-semibold tabular-nums text-brass">
                {{ folder.unread }}
              </span>
            </RouterLink>
          </li>
        </ul>
        <form v-if="addingFolder" class="mt-2 space-y-2 px-2.5" @submit.prevent="addFolder">
          <input
            v-model="folderName"
            class="h-8 w-full rounded-control border border-lagoon-line bg-transparent px-2 text-sm text-lagoon-ink"
            placeholder="Folder name"
            aria-label="New folder name"
          />
          <button type="submit" class="text-sm font-semibold text-brass">Add folder</button>
        </form>
        <button
          v-else
          type="button"
          class="mt-2 px-2.5 text-sm text-lagoon-muted hover:text-lagoon-ink"
          @click="addingFolder = true"
        >
          Add folder
        </button>
      </div>
      <div v-if="!railCollapsed" class="mt-5">
        <p class="px-2.5 text-xs font-semibold text-lagoon-muted">Labels</p>
        <p v-if="mail.labels.length === 0" class="px-2.5 pt-2 text-sm text-lagoon-muted">None yet</p>
        <ul v-else class="mt-1 space-y-0.5">
          <li v-for="label in mail.labels" :key="label.id">
            <button
              type="button"
              class="flex h-8 w-full items-center gap-2 rounded-control px-2.5 text-left text-sm text-lagoon-ink hover:bg-lagoon-active"
              :class="mail.activeLabelId === label.id ? 'bg-lagoon-active' : ''"
              :aria-pressed="mail.activeLabelId === label.id"
              @click="chooseLabel(label.id)"
            >
              <span class="size-2 shrink-0 rounded-full" :class="railTone[label.tone]" />
              <span class="truncate">{{ label.name }}</span>
            </button>
          </li>
        </ul>
        <form v-if="addingLabel" class="mt-2 space-y-2 px-2.5" @submit.prevent="addLabel">
          <input
            v-model="labelName"
            class="h-8 w-full rounded-control border border-lagoon-line bg-transparent px-2 text-sm text-lagoon-ink"
            placeholder="Label name"
            aria-label="New label name"
          />
          <button type="submit" class="text-sm font-semibold text-brass">Add label</button>
        </form>
        <button
          v-else
          type="button"
          class="mt-2 px-2.5 text-sm text-lagoon-muted hover:text-lagoon-ink"
          @click="addingLabel = true"
        >
          Add label
        </button>
      </div>
    </nav>

    <div class="px-3 pb-4">
      <RouterLink
        to="/contacts"
        class="flex h-9 items-center gap-3 rounded-control text-sm text-lagoon-ink hover:bg-lagoon-active"
        :class="[
          route.path === '/contacts' ? 'bg-lagoon-active' : '',
          railCollapsed ? 'justify-center px-0' : 'px-2.5',
        ]"
        :aria-current="route.path === '/contacts' ? 'page' : undefined"
        :title="railCollapsed ? 'Contacts' : undefined"
        @click="ui.closeMobile()"
      >
        <BookUser class="size-4 shrink-0" :class="route.path === '/contacts' ? 'text-brass' : 'text-lagoon-muted'" />
        <span v-show="!railCollapsed">Contacts</span>
      </RouterLink>
      <p v-show="!railCollapsed" class="px-2.5 pt-4 text-xs text-lagoon-muted">
        IT KAMFANY LIMITED
      </p>
    </div>
  </aside>
</template>
