<script setup lang="ts">
import { Flag, Paperclip, Star } from "@lucide/vue";
import { computed } from "vue";
import type { LabelTone, MailLabel, MailMessage } from "../../types/mail";
import { formatListDate } from "../../utils/format-mail";
import { deliveryLabel } from "../../utils/message-html";

const props = defineProps<{
  message: MailMessage;
  labels: MailLabel[];
  selected: boolean;
  open: boolean;
}>();

const emit = defineEmits<{
  open: [];
  toggle: [];
  star: [];
}>();

const toneClass: Record<LabelTone, string> = {
  brass: "bg-brass",
  paper: "bg-lagoon",
  mist: "bg-muted",
};

const chips = computed(() =>
  props.labels.filter((label) => props.message.labelIds.includes(label.id)),
);
</script>

<template>
  <div
    class="grid cursor-pointer grid-cols-[1.25rem_1.25rem_minmax(0,1fr)] items-start gap-x-2.5 border-b border-line px-4 py-3.5 text-left hover:bg-canvas"
    :class="open ? 'bg-canvas' : ''"
    role="option"
    :aria-selected="open"
    tabindex="0"
    @click="emit('open')"
    @keydown.enter.prevent="emit('open')"
    @keydown.space.prevent="emit('open')"
  >
    <input
      type="checkbox"
      class="mt-1 size-4 accent-brass"
      :checked="selected"
      :aria-label="`Select ${message.subject}`"
      @click.stop
      @change="emit('toggle')"
    />
    <button
      type="button"
      class="mt-0.5 grid size-5 place-items-center rounded-full"
      :aria-pressed="message.isStarred"
      :aria-label="message.isStarred ? `Unstar ${message.subject}` : `Star ${message.subject}`"
      @click.stop="emit('star')"
    >
      <Star
        class="size-4"
        :class="message.isStarred ? 'fill-brass text-brass' : 'text-muted'"
      />
    </button>
    <div class="min-w-0">
      <div class="flex items-baseline gap-2">
        <span
          class="size-1.5 shrink-0 self-center rounded-full"
          :class="message.isRead ? 'bg-transparent' : 'bg-brass'"
          aria-hidden="true"
        />
        <span class="sr-only">{{ message.isRead ? "Read" : "Unread" }}</span>
        <span
          class="min-w-0 flex-1 truncate"
          :class="message.isRead ? 'font-medium' : 'font-semibold'"
        >
          {{ message.from.name }}
        </span>
        <span v-if="deliveryLabel(message.status) && message.status !== 'received'" class="shrink-0 text-xs text-muted">
          {{ deliveryLabel(message.status) }}
        </span>
        <time
          class="shrink-0 text-xs tabular-nums text-muted"
          :datetime="message.receivedAt"
        >
          {{ formatListDate(message.receivedAt) }}
        </time>
      </div>
      <p
        class="truncate pl-3.5 text-sm"
        :class="message.isRead ? 'text-ink' : 'font-semibold text-ink'"
      >
        {{ message.subject }}
      </p>
      <p class="truncate pl-3.5 text-sm text-muted">
        <span>{{ message.from.email }}</span>
        <template v-if="message.mailboxAddress && message.mailboxAddress !== message.from.email">
          <span aria-hidden="true"> · </span>
          <span>{{ message.mailboxAddress }}</span>
        </template>
        <span aria-hidden="true"> · </span>
        <span>{{ message.preview }}</span>
      </p>
      <div
        v-if="message.hasAttachments || message.attachments.length || message.isImportant || chips.length"
        class="mt-1 flex min-w-0 items-center gap-2 pl-3.5"
      >
        <Paperclip
          v-if="message.hasAttachments || message.attachments.length"
          class="size-3.5 shrink-0 text-muted"
        />
        <span v-if="message.hasAttachments || message.attachments.length" class="sr-only">Has an attachment</span>
        <Flag v-if="message.isImportant" class="size-3.5 shrink-0 text-brass" />
        <span v-if="message.isImportant" class="sr-only">Important</span>
        <span
          v-for="label in chips"
          :key="label.id"
          class="inline-flex min-w-0 items-center gap-1 truncate text-xs text-muted"
        >
          <span class="size-1.5 shrink-0 rounded-full" :class="toneClass[label.tone]" />
          {{ label.name }}
        </span>
      </div>
    </div>
  </div>
</template>
