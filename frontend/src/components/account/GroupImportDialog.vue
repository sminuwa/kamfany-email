<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";
import { importGroupFile, type GroupImport, type MailGroup } from "../../services/groups.service";
import { errorMessage } from "../../services/http";

const open = defineModel<boolean>({ required: true });
const props = defineProps<{ group: MailGroup | null }>();
const emit = defineEmits<{ imported: [result: GroupImport] }>();

const dialog = ref<HTMLElement | null>(null);
const fileInput = ref<HTMLInputElement | null>(null);
const file = ref<File | null>(null);
const notice = ref("");
const failure = ref("");
const pending = ref(false);

watch(open, async (isOpen) => {
  failure.value = "";
  notice.value = "";
  pending.value = false;
  file.value = null;
  if (isOpen) {
    await nextTick();
    fileInput.value?.focus();
  }
});

onKeyStroke("Escape", () => {
  if (open.value && !pending.value) {
    open.value = false;
  }
});

function trapTab(event: KeyboardEvent) {
  if (event.key !== "Tab" || !dialog.value) {
    return;
  }
  const nodes = [...dialog.value.querySelectorAll<HTMLElement>("button, input, textarea, select, a[href]")].filter(
    (node) => !node.hasAttribute("disabled"),
  );
  if (!nodes.length) {
    return;
  }
  const first = nodes[0];
  const last = nodes[nodes.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function chooseFile(event: Event) {
  const input = event.target as HTMLInputElement;
  file.value = input.files?.[0] ?? null;
  failure.value = "";
  notice.value = "";
}

function summary(result: GroupImport): string {
  const parts = [`Added ${result.added} ${result.added === 1 ? "mailbox" : "mailboxes"}.`];
  if (result.alreadyInGroup > 0) {
    parts.push(`${result.alreadyInGroup} already in the group.`);
  }
  if (result.unknownCount > 0) {
    const listed = result.unknown.join(", ");
    parts.push(
      result.unknownCount === result.unknown.length
        ? `${result.unknownCount} ${result.unknownCount === 1 ? "address has" : "addresses have"} no mailbox: ${listed}.`
        : `${result.unknownCount} addresses have no mailbox, including ${listed}.`,
    );
  }
  if (result.notAdded > 0) {
    parts.push(`${result.notAdded} were left out because a group holds at most 50 mailboxes.`);
  }
  return parts.join(" ");
}

async function submit() {
  if (!props.group || pending.value) {
    return;
  }
  if (!file.value) {
    failure.value = "Choose a CSV or Excel file.";
    return;
  }
  pending.value = true;
  failure.value = "";
  notice.value = "";
  try {
    const result = await importGroupFile(props.group.id, file.value);
    notice.value = summary(result);
    file.value = null;
    if (fileInput.value) {
      fileInput.value.value = "";
    }
    emit("imported", result);
  } catch (error) {
    failure.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 grid place-items-center bg-ink/40 px-4 py-8"
      @click.self="!pending && (open = false)"
    >
      <div
        ref="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="group-import-title"
        class="km-card w-full max-w-md p-6 text-ink"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="group-import-title" class="text-xl font-semibold tracking-[-0.03em]">Import mailboxes</h2>
          <button
            type="button"
            class="grid size-8 shrink-0 place-items-center rounded-control text-muted hover:bg-canvas hover:text-ink"
            aria-label="Close"
            :disabled="pending"
            @click="open = false"
          >
            <X class="size-4" />
          </button>
        </div>
        <p class="mt-2 text-sm leading-relaxed text-muted">
          Add addresses to <span class="font-medium text-ink">{{ group?.name }}</span>. Use a CSV or Excel file with a
          column named email, address, or mailbox, or one address on each line. Each address must already be a mailbox.
        </p>
        <form class="mt-5 space-y-3" @submit.prevent="submit">
          <label class="block text-sm font-medium">
            File
            <input
              ref="fileInput"
              class="mt-1.5 block w-full text-sm"
              type="file"
              accept=".csv,.xlsx,.xls,text/csv,application/vnd.ms-excel,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
              @change="chooseFile"
            />
          </label>
          <p v-if="file" class="text-sm text-muted">{{ file.name }}</p>
          <p v-if="failure" class="text-sm leading-relaxed text-danger" role="alert">{{ failure }}</p>
          <p v-else-if="notice" class="text-sm leading-relaxed text-muted" role="status">{{ notice }}</p>
          <div class="flex items-center justify-end gap-2 pt-1">
            <button
              type="button"
              class="inline-flex h-10 items-center rounded-full px-4 text-sm font-semibold text-muted hover:bg-canvas hover:text-ink"
              :disabled="pending"
              @click="open = false"
            >
              Close
            </button>
            <button type="submit" class="km-action" :disabled="pending">
              {{ pending ? "Importing…" : "Import" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
