<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";
import { field } from "../../pages/auth/account-form";
import { errorMessage } from "../../services/http";
import type { MailSignature } from "../../services/compose.service";

const open = defineModel<boolean>({ required: true });
const name = defineModel<string>("name", { required: true });
const body = defineModel<string>("body", { required: true });

const props = defineProps<{
  signatures: MailSignature[];
  save: () => Promise<boolean>;
  remove: (id: string) => Promise<void>;
}>();

const dialog = ref<HTMLElement | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);
const notice = ref("");
const pending = ref(false);

watch(open, async (isOpen) => {
  notice.value = "";
  pending.value = false;
  if (!isOpen) {
    return;
  }
  await nextTick();
  nameInput.value?.focus();
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

async function submit() {
  if (pending.value) {
    return;
  }
  notice.value = "";
  pending.value = true;
  try {
    const saved = await props.save();
    if (saved) {
      open.value = false;
    }
  } catch (error) {
    notice.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}

async function removeSignature(id: string) {
  notice.value = "";
  try {
    await props.remove(id);
  } catch (error) {
    notice.value = errorMessage(error);
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
        aria-labelledby="signature-title"
        class="km-card w-full max-w-md p-6 text-ink"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="signature-title" class="text-xl font-semibold tracking-[-0.03em]">Add a signature</h2>
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
          The signature is added to the end of messages you send from this mailbox.
        </p>
        <form class="mt-5 space-y-3" @submit.prevent="submit">
          <input
            ref="nameInput"
            v-model="name"
            required
            maxlength="80"
            placeholder="Signature name"
            :class="[field, 'bg-canvas placeholder:text-muted']"
            aria-label="Signature name"
          />
          <textarea
            v-model="body"
            rows="4"
            required
            placeholder="How you sign your mail"
            class="h-auto w-full rounded-control border border-line bg-canvas px-3 py-2 text-sm text-ink outline-none placeholder:text-muted focus:border-lagoon"
            aria-label="Signature text"
          />
          <ul v-if="signatures.length" class="space-y-1">
            <li v-for="item in signatures" :key="item.id" class="flex items-center justify-between gap-3 text-sm">
              <span class="min-w-0 truncate">{{ item.name }}</span>
              <button type="button" class="shrink-0 text-muted hover:text-danger" @click="removeSignature(item.id)">
                Delete
              </button>
            </li>
          </ul>
          <p v-if="notice" class="text-sm leading-relaxed text-danger" role="alert">{{ notice }}</p>
          <div class="flex items-center justify-end gap-2 pt-1">
            <button
              type="button"
              class="inline-flex h-10 items-center rounded-full px-4 text-sm font-semibold text-muted hover:bg-canvas hover:text-ink"
              :disabled="pending"
              @click="open = false"
            >
              Cancel
            </button>
            <button type="submit" class="km-action" :disabled="pending">
              {{ pending ? "Saving…" : "Save signature" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
