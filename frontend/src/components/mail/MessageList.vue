<script setup lang="ts">
import { X } from "@lucide/vue";
import { computed, ref, watch, watchEffect } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMailboxStore } from "../../stores/mailboxes";
import { useMailStore } from "../../stores/messages";
import { useUiStore } from "../../stores/ui";
import type { ComposeKind, LabelTone, MailMessage, OrganizationalFolder } from "../../types/mail";
import { folderByKey } from "../../utils/folders";
import { filterMessages, isSnoozed, PAGE_SIZE } from "../../utils/mail-query";
import { filingAction, readActions, starAction } from "../../utils/message-menu";
import LabelMenu from "./LabelMenu.vue";
import MessageContextMenu from "./MessageContextMenu.vue";
import MessageRow from "./MessageRow.vue";
import MoveMenu from "./MoveMenu.vue";
import SnoozeMenu from "./SnoozeMenu.vue";

const emit = defineEmits<{
  open: [id: string];
}>();

const route = useRoute();
const router = useRouter();
const ui = useUiStore();
const mail = useMailStore();
const boxes = useMailboxStore();
const limit = ref(PAGE_SIZE);
const allBox = ref<HTMLInputElement | null>(null);
const menu = ref<{ id: string; x: number; y: number } | null>(null);
const folderName = ref("");
const folderColor = ref<LabelTone | null>("brass");
const labelName = ref("");

const customFolder = computed(() =>
  typeof route.params.folderId === "string"
    ? (mail.folders.find((item) => item.id === route.params.folderId) ?? null)
    : null,
);

const folder = computed(() => {
  if (customFolder.value) {
    return {
      key: customFolder.value.id,
      label: customFolder.value.name,
      path: `/folders/${customFolder.value.id}`,
      description: "Messages filed in this folder.",
    };
  }
  return folderByKey(typeof route.meta.folder === "string" ? route.meta.folder : undefined);
});
const openId = computed(() => (typeof route.query.m === "string" ? route.query.m : ""));

const filtered = computed(() => {
  if (mail.searchActive) {
    return mail.results;
  }
  return filterMessages(mail.messages, {
    folder: customFolder.value ? "custom" : folder.value.key,
    folderId: customFolder.value?.id ?? null,
    query: "",
    filters: mail.filters,
    labelId: mail.activeLabelId,
  });
});

const visible = computed(() => filtered.value.slice(0, limit.value));
const selectedInView = computed(() =>
  filtered.value.filter((message) => mail.selectedIds.includes(message.id)),
);
const allSelected = computed(
  () => filtered.value.length > 0 && selectedInView.value.length === filtered.value.length,
);
const someSelected = computed(() => selectedInView.value.length > 0 && !allSelected.value);
const activeLabel = computed(
  () => mail.labels.find((label) => label.id === mail.activeLabelId) ?? null,
);
const narrowed = computed(
  () =>
    mail.searchActive ||
    mail.filters.unread ||
    mail.filters.attachments ||
    mail.filters.important ||
    mail.activeLabelId !== null,
);

const heading = computed(() => (mail.searchActive ? "Search" : folder.value.label));
const unreadCount = computed(() => filtered.value.filter((message) => !message.isRead).length);

const summary = computed(() => {
  const total = filtered.value.length;
  if (mail.searchNote) {
    return mail.searchNote;
  }
  if (narrowed.value) {
    return total === 1 ? "1 match" : `${total} matches`;
  }
  return total === 1 ? "1 message" : `${total} messages`;
});

const emptyTitle = computed(() => {
  if (mail.searchActive) {
    return "Nothing matches that search.";
  }
  if (narrowed.value) {
    return `Nothing in ${folder.value.label} matches these filters.`;
  }
  return `${folder.value.label} is clear`;
});

const filingLabel = computed(() =>
  folder.value.key === "archive" || folder.value.key === "spam" || folder.value.key === "trash"
    ? "Move to inbox"
    : "Archive",
);

const menuMessage = computed(
  () => filtered.value.find((message) => message.id === menu.value?.id) ?? null,
);
const menuTargets = computed(() => {
  if (!menuMessage.value) {
    return [];
  }
  if (mail.selectedIds.includes(menuMessage.value.id)) {
    return selectedInView.value;
  }
  return [menuMessage.value];
});
const menuFiling = computed(() => filingAction(menuTargets.value));
const menuStar = computed(() => starAction(menuTargets.value));
const menuRead = computed(() => readActions(menuTargets.value));
const menuCanSnooze = computed(
  () =>
    Boolean(menuMessage.value) &&
    menuMessage.value?.status !== "draft" &&
    (menuMessage.value?.folder === "inbox" ||
      menuTargets.value.some((message) => isSnoozed(message, Date.now()))),
);
const menuUnsnooze = computed(() =>
  menuTargets.value.some((message) => isSnoozed(message, Date.now())),
);
const menuFolders = computed(() =>
  mail.folders
    .filter((item) => !item.systemKey)
    .map((item) => ({ id: item.id, name: item.name })),
);

watch(
  () =>
    [
      folder.value.key,
      ui.searchQuery,
      mail.activeLabelId,
      mail.filters.unread,
      mail.filters.attachments,
      mail.filters.important,
    ].join("|"),
  () => {
    limit.value = PAGE_SIZE;
    menu.value = null;
  },
);

watch(filtered, (list) => {
  const allowed = new Set(list.map((message) => message.id));
  const next = mail.selectedIds.filter((id) => allowed.has(id));
  if (next.length !== mail.selectedIds.length) {
    mail.selectAll(next);
  }
});

watchEffect(() => {
  if (allBox.value) {
    allBox.value.indeterminate = someSelected.value;
  }
});

function openMenu(message: MailMessage, event: MouseEvent) {
  if (!mail.selectedIds.includes(message.id)) {
    mail.selectAll([message.id]);
  }
  menu.value = { id: message.id, x: event.clientX, y: event.clientY };
}

function menuIds(): string[] {
  return menuTargets.value.map((message) => message.id);
}

function closeMenu() {
  menu.value = null;
}

function openFromMenu() {
  const id = menu.value?.id;
  closeMenu();
  if (id) {
    emit("open", id);
  }
}

function replyFromMenu(kind: ComposeKind) {
  const id = menu.value?.id;
  closeMenu();
  if (id) {
    ui.openComposer({ kind, messageId: id, draftId: null });
  }
}

function readFromMenu(read: boolean) {
  const ids = menuIds();
  closeMenu();
  void mail.setRead(ids, read);
}

function starFromMenu() {
  const ids = menuIds();
  const starred = menuStar.value.starred;
  closeMenu();
  void mail.setStar(ids, starred);
}

function snoozeFromMenu(until: string | null) {
  const ids = menuIds();
  closeMenu();
  void mail.snooze(ids, until);
}

function fileFromMenu() {
  const ids = menuIds();
  const target = menuFiling.value.target;
  closeMenu();
  void mail.moveTo(ids, target);
}

function moveFromMenu(target: string) {
  const ids = menuIds();
  closeMenu();
  void mail.moveTo(ids, target);
}

function labelFromMenu(id: string) {
  const ids = menuIds();
  closeMenu();
  void mail.addLabel(ids, id);
}

function deleteFromMenu() {
  const ids = menuIds();
  closeMenu();
  void mail.deleteMessages(ids);
}

function onSelectAll(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  mail.selectAll(checked ? filtered.value.map((message) => message.id) : []);
}

function selectedIds(): string[] {
  return selectedInView.value.map((message) => message.id);
}

function fileSelected() {
  const destination: OrganizationalFolder =
    folder.value.key === "archive" || folder.value.key === "spam" || folder.value.key === "trash"
      ? "inbox"
      : "archive";
  mail.moveTo(selectedIds(), destination);
}

function starSelected() {
  const starred = selectedInView.value.every((message) => message.isStarred);
  void mail.setStar(selectedIds(), !starred);
}

function showMore() {
  if (visible.value.length < filtered.value.length) {
    limit.value += PAGE_SIZE;
    return;
  }
  void mail.loadOlder();
}

function saveCustomFolder() {
  if (!customFolder.value) {
    return;
  }
  void mail.saveFolder(customFolder.value.id, folderName.value, folderColor.value);
}

async function removeCustomFolder() {
  if (!customFolder.value) {
    return;
  }
  const id = customFolder.value.id;
  await mail.removeFolder(id);
  await router.push("/");
}

function saveActiveLabel() {
  if (!activeLabel.value) {
    return;
  }
  void mail.saveLabel(activeLabel.value.id, labelName.value, activeLabel.value.tone);
}

watch(customFolder, (folder) => {
  folderName.value = folder?.name ?? "";
  folderColor.value = folder?.color ?? "brass";
}, { immediate: true });

watch(activeLabel, (label) => {
  labelName.value = label?.name ?? "";
}, { immediate: true });
</script>

<template>
  <section class="desk-chrome flex h-full min-h-0 flex-col" :aria-label="folder.label">
    <header class="border-b border-line px-4 py-3">
      <div class="flex min-h-10 items-center gap-3">
        <input
          ref="allBox"
          type="checkbox"
          class="size-4 accent-brass"
          :checked="allSelected"
          :disabled="filtered.length === 0"
          aria-label="Select all"
          @change="onSelectAll"
        />
        <div class="flex min-w-0 flex-1 items-baseline gap-2">
          <h1 class="truncate text-2xl font-semibold tracking-[-0.03em]">{{ heading }}</h1>
          <p
            class="text-sm text-muted"
            :class="mail.searchNote ? 'min-w-0 truncate' : 'shrink-0'"
            aria-live="polite"
          >{{ summary }}</p>
          <span
            v-if="!mail.searchActive && !narrowed && unreadCount > 0"
            class="shrink-0 rounded-full bg-brass px-2 py-0.5 text-[11px] font-semibold tabular-nums text-brass-ink"
          >
            {{ unreadCount }} unread
          </span>
        </div>
        <button
          v-if="activeLabel"
          type="button"
          class="inline-flex shrink-0 items-center gap-1 rounded-full bg-canvas px-2 py-1 text-xs font-medium"
          @click="mail.clearLabelFilter()"
        >
          {{ activeLabel.name }}
          <X class="size-3" />
          <span class="sr-only">Clear label filter</span>
        </button>
      </div>
      <p v-if="mail.notice || boxes.notice" class="mt-2 rounded-control bg-canvas px-3 py-2 text-sm leading-relaxed text-muted">
        {{ mail.notice || boxes.notice }}
      </p>
      <form
        v-if="customFolder"
        class="mt-2 flex flex-wrap items-center gap-2"
        @submit.prevent="saveCustomFolder"
      >
        <input v-model="folderName" class="h-8 rounded-control border border-line bg-canvas px-2 text-sm" aria-label="Folder name" />
        <select v-model="folderColor" class="h-8 rounded-control border border-line bg-canvas px-2 text-sm" aria-label="Folder color">
          <option value="brass">Brass</option>
          <option value="paper">Paper</option>
          <option value="mist">Mist</option>
        </select>
        <button type="submit" class="h-8 rounded-control px-2 text-sm hover:bg-canvas">Save folder</button>
        <button type="button" class="h-8 rounded-control px-2 text-sm text-danger hover:bg-canvas" @click="removeCustomFolder">
          Delete folder
        </button>
      </form>
      <form v-if="activeLabel" class="mt-2 flex flex-wrap items-center gap-2" @submit.prevent="saveActiveLabel">
        <input v-model="labelName" class="h-8 rounded-control border border-line bg-canvas px-2 text-sm" aria-label="Label name" />
        <button type="submit" class="h-8 rounded-control px-2 text-sm hover:bg-canvas">Save label</button>
        <button type="button" class="h-8 rounded-control px-2 text-sm text-danger hover:bg-canvas" @click="mail.removeNamedLabel(activeLabel.id)">
          Delete label
        </button>
      </form>
      <div v-if="selectedInView.length" class="flex flex-wrap items-center gap-1 pt-2">
        <p class="px-1 text-xs font-medium text-muted">{{ selectedInView.length }} selected</p>
        <button type="button" class="h-8 rounded-control px-2 text-sm hover:bg-canvas" @click="fileSelected">
          {{ filingLabel }}
        </button>
        <button
          type="button"
          class="h-8 rounded-control px-2 text-sm text-danger hover:bg-canvas"
          @click="mail.deleteMessages(selectedIds())"
        >
          Delete
        </button>
        <button type="button" class="h-8 rounded-control px-2 text-sm hover:bg-canvas" @click="mail.setRead(selectedIds(), true)">
          Read
        </button>
        <button type="button" class="h-8 rounded-control px-2 text-sm hover:bg-canvas" @click="mail.setRead(selectedIds(), false)">
          Unread
        </button>
        <button type="button" class="h-8 rounded-control px-2 text-sm hover:bg-canvas" @click="starSelected">
          Star
        </button>
        <SnoozeMenu
          v-if="folder.key === 'inbox' || folder.key === 'snoozed'"
          :unsnooze="folder.key === 'snoozed' || selectedInView.some((message) => isSnoozed(message, Date.now()))"
          @snooze="mail.snooze(selectedIds(), $event)"
        />
        <MoveMenu @move="mail.moveTo(selectedIds(), $event)" />
        <LabelMenu @label="mail.addLabel(selectedIds(), $event)" />
      </div>
    </header>

    <div
      class="min-h-0 flex-1 overflow-y-auto rounded-b-panel"
      role="listbox"
      :aria-label="`${folder.label} messages`"
      @scroll="closeMenu"
    >
      <div v-if="filtered.length === 0" class="grid h-full place-items-center px-6 py-16 text-center">
        <div class="max-w-[36ch]">
          <p class="text-base font-semibold">{{ emptyTitle }}</p>
          <p class="mt-2 text-sm leading-relaxed text-muted">{{ folder.description }}</p>
        </div>
      </div>
      <template v-else>
        <MessageRow
          v-for="message in visible"
          :key="message.id"
          :message="message"
          :labels="mail.labels"
          :selected="mail.selectedIds.includes(message.id)"
          :open="openId === message.id"
          @open="emit('open', message.id)"
          @toggle="mail.toggleSelected(message.id)"
          @star="mail.toggleStar(message.id)"
          @menu="openMenu(message, $event)"
        />
        <div v-if="visible.length < filtered.length || mail.nextCursor" class="p-3">
          <button
            type="button"
            class="h-10 w-full rounded-control border border-line text-sm font-semibold hover:bg-canvas"
            @click="showMore"
          >
            {{ visible.length < filtered.length ? "Show more" : "Load older" }}
          </button>
        </div>
      </template>
    </div>
    <MessageContextMenu
      v-if="menu && menuMessage"
      :x="menu.x"
      :y="menu.y"
      :subject="menuMessage.subject"
      :count="menuTargets.length"
      :draft="menuMessage.status === 'draft'"
      :filing-label="menuFiling.label"
      :filing-target="menuFiling.target"
      :star-label="menuStar.label"
      :read-actions="menuRead"
      :folders="menuFolders"
      :labels="mail.labels"
      :can-snooze="menuCanSnooze"
      :unsnooze="menuUnsnooze"
      @close="closeMenu"
      @open="openFromMenu"
      @reply="replyFromMenu"
      @read="readFromMenu"
      @star="starFromMenu"
      @snooze="snoozeFromMenu"
      @file="fileFromMenu"
      @move="moveFromMenu"
      @label="labelFromMenu"
      @remove="deleteFromMenu"
    />
  </section>
</template>
