<script setup lang="ts">
import {
  Archive,
  ChevronLeft,
  Inbox,
  Mail,
  Printer,
  Reply,
  ReplyAll,
  Forward,
  Star,
  Trash2,
  Paperclip,
} from "@lucide/vue";
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useTheme } from "../../composables/useTheme";
import { useBrandStore } from "../../stores/brand";
import { useMailStore } from "../../stores/messages";
import { useUiStore } from "../../stores/ui";
import type { MailMessage, OrganizationalFolder } from "../../types/mail";
import { emailSrcdoc } from "../../utils/email-document";
import { formatFileSize, formatMessageDate, formatPeople, scanNote } from "../../utils/format-mail";
import { deliveryLabel } from "../../utils/message-html";
import { sanitizeEmailHtml } from "../../utils/sanitize-email";
import LabelMenu from "./LabelMenu.vue";
import MoveMenu from "./MoveMenu.vue";

const props = defineProps<{
  message: MailMessage;
}>();

const emit = defineEmits<{
  close: [];
}>();

const brand = useBrandStore();
const mail = useMailStore();
const ui = useUiStore();
const route = useRoute();
const router = useRouter();
const { resolved } = useTheme();

const sanitized = computed(() =>
  props.message.bodyHtml ? sanitizeEmailHtml(props.message.bodyHtml) : "",
);
const srcdoc = computed(() => emailSrcdoc(sanitized.value, resolved.value));
const filing = computed(() => {
  const folder = props.message.folder;
  if (folder === "archive" || folder === "spam" || folder === "trash") {
    return { folder: "inbox" as OrganizationalFolder, label: "Move to inbox" };
  }
  return { folder: "archive" as OrganizationalFolder, label: "Archive" };
});

function ids(): string[] {
  return [props.message.id];
}

function reply(kind: "reply" | "reply-all" | "forward") {
  ui.openComposer({ kind, messageId: props.message.id, draftId: null });
}

function fileMessage() {
  mail.moveTo(ids(), filing.value.folder);
}

function printMessage() {
  window.print();
}

function openThread(id: string) {
  void router.push({ query: { ...route.query, m: id } });
}
</script>

<template>
  <article class="reading-pane flex h-full min-h-0 flex-1 flex-col bg-surface">
    <header class="desk-chrome flex flex-wrap items-center gap-1 border-b border-line px-2 py-2">
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas min-[960px]:hidden"
        aria-label="Back to list"
        @click="emit('close')"
      >
        <ChevronLeft class="size-4" />
      </button>
      <button type="button" class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas" aria-label="Reply" @click="reply('reply')">
        <Reply class="size-4" />
      </button>
      <button type="button" class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas" aria-label="Reply all" @click="reply('reply-all')">
        <ReplyAll class="size-4" />
      </button>
      <button type="button" class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas" aria-label="Forward" @click="reply('forward')">
        <Forward class="size-4" />
      </button>
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas"
        :aria-label="filing.label"
        @click="fileMessage"
      >
        <Inbox v-if="filing.folder === 'inbox'" class="size-4" />
        <Archive v-else class="size-4" />
      </button>
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas hover:text-danger"
        aria-label="Delete"
        @click="mail.deleteMessages(ids())"
      >
        <Trash2 class="size-4" />
      </button>
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas"
        :aria-pressed="message.isStarred"
        :aria-label="message.isStarred ? 'Unstar' : 'Star'"
        @click="mail.toggleStar(message.id)"
      >
        <Star class="size-4" :class="message.isStarred ? 'fill-brass text-brass' : ''" />
      </button>
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas"
        aria-label="Mark unread"
        @click="mail.setRead(ids(), false)"
      >
        <Mail class="size-4" />
      </button>
      <MoveMenu compact @move="mail.moveTo(ids(), $event)" />
      <LabelMenu compact @label="mail.addLabel(ids(), $event)" />
      <button
        type="button"
        class="grid size-9 place-items-center rounded-control text-ink hover:bg-canvas"
        aria-label="Print"
        @click="printMessage"
      >
        <Printer class="size-4" />
      </button>
    </header>

    <div class="desk-chrome shrink-0 border-b border-line px-5 py-4">
      <h2 class="text-2xl font-semibold tracking-[-0.03em]">{{ message.subject }}</h2>
      <p v-if="deliveryLabel(message.status)" class="mt-1 text-xs font-semibold uppercase tracking-[0.04em] text-muted">
        {{ deliveryLabel(message.status) }}
      </p>
      <ul v-if="mail.thread.length > 1" class="mt-3 space-y-1">
        <li v-for="item in mail.thread" :key="item.id">
          <button
            type="button"
            class="w-full truncate rounded-control px-2 py-1 text-left text-sm hover:bg-canvas"
            :class="item.id === message.id ? 'bg-canvas font-semibold' : 'text-muted'"
            @click="openThread(item.id)"
          >
            {{ item.fromName }} · {{ item.subject }}
          </button>
        </li>
      </ul>
      <div class="mt-3 flex items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="truncate font-semibold">{{ message.from.name }}</p>
          <p class="truncate text-sm text-muted">{{ message.from.email }}</p>
        </div>
        <time class="shrink-0 text-right text-xs text-muted" :datetime="message.receivedAt">
          {{ formatMessageDate(message.receivedAt) }}
        </time>
      </div>
      <dl class="mt-3 space-y-1 text-sm">
        <div class="flex gap-2">
          <dt class="w-8 shrink-0 text-muted">To</dt>
          <dd class="min-w-0 break-words">{{ formatPeople(message.to) }}</dd>
        </div>
        <div v-if="message.cc.length" class="flex gap-2">
          <dt class="w-8 shrink-0 text-muted">Cc</dt>
          <dd class="min-w-0 break-words">{{ formatPeople(message.cc) }}</dd>
        </div>
        <div v-if="message.bcc.length" class="flex gap-2">
          <dt class="w-8 shrink-0 text-muted">Bcc</dt>
          <dd class="min-w-0 break-words">{{ formatPeople(message.bcc) }}</dd>
        </div>
      </dl>
      <p v-if="message.hasAttachments && message.attachments.length === 0" class="mt-3 text-sm text-muted">
        This message has attachments. The files are not on this desk yet.
      </p>
      <ul v-if="message.attachments.length" class="mt-3 flex flex-wrap gap-2">
        <li v-for="file in message.attachments" :key="file.id" class="rounded-control border border-line px-2.5 py-1.5 text-sm">
          <img
            v-if="file.scanStatus !== 'rejected' && file.scanStatus !== 'failed' && file.mimeType.startsWith('image/') && file.mimeType !== 'image/svg+xml'"
            class="mb-2 max-h-40 rounded-control"
            :src="`/api/v1/attachments/${file.id}`"
            :alt="file.filename"
          />
          <a
            v-if="file.scanStatus !== 'rejected' && file.scanStatus !== 'failed'"
            class="flex items-center gap-2 hover:text-brass-ink"
            :href="`/api/v1/attachments/${file.id}`"
          >
            <Paperclip class="size-3.5 text-muted" />
            <span>{{ file.filename }}</span>
            <span class="text-muted">{{ formatFileSize(file.sizeBytes) }}</span>
            <span v-if="scanNote(file.scanStatus)" class="text-muted">{{ scanNote(file.scanStatus) }}</span>
          </a>
          <p v-else class="flex items-center gap-2">
            <Paperclip class="size-3.5 text-muted" />
            <span>{{ file.filename }}</span>
            <span class="text-muted">{{ scanNote(file.scanStatus) }}</span>
          </p>
        </li>
      </ul>
    </div>

    <iframe
      v-if="message.bodyHtml"
      class="desk-chrome min-h-0 w-full flex-1 border-0 bg-surface"
      sandbox=""
      referrerpolicy="no-referrer"
      :srcdoc="srcdoc"
      title="Message content"
    />
    <div v-else class="desk-chrome min-h-0 flex-1 overflow-y-auto px-5 py-4">
      <pre class="whitespace-pre-wrap font-sans text-[15px] leading-relaxed">{{ message.bodyText }}</pre>
    </div>

    <div class="print-sheet px-8 py-6">
      <p class="text-xs font-semibold tracking-[0.04em]">{{ brand.name }}</p>
      <h1 class="mt-3 text-2xl font-semibold tracking-[-0.02em]">{{ message.subject }}</h1>
      <p class="mt-4 font-semibold">{{ message.from.name }}</p>
      <p class="text-sm">{{ message.from.email }}</p>
      <p class="mt-2 text-sm">To {{ formatPeople(message.to) }}</p>
      <p v-if="message.cc.length" class="text-sm">Cc {{ formatPeople(message.cc) }}</p>
      <p v-if="message.bcc.length" class="text-sm">Bcc {{ formatPeople(message.bcc) }}</p>
      <p class="mt-2 text-sm">{{ formatMessageDate(message.receivedAt) }}</p>
      <div v-if="message.bodyHtml" class="mt-6 text-[15px] leading-relaxed" v-html="sanitized" />
      <pre v-else class="mt-6 whitespace-pre-wrap font-sans text-[15px] leading-relaxed">{{ message.bodyText }}</pre>
    </div>
  </article>
</template>
