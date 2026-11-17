<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";
import { errorMessage } from "../../services/http";
import { useBrandStore } from "../../stores/brand";
import { useMailboxStore } from "../../stores/mailboxes";
import { field } from "../../pages/auth/account-form";

const open = defineModel<boolean>({ required: true });
const emit = defineEmits<{ registered: [message: string] }>();

const brand = useBrandStore();
const boxes = useMailboxStore();
const dialog = ref<HTMLElement | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);
const mailboxAddress = ref("");
const mailboxName = ref("");
const notice = ref("");
const pending = ref(false);

watch(open, async (isOpen) => {
  notice.value = "";
  pending.value = false;
  if (!isOpen) {
    mailboxAddress.value = "";
    mailboxName.value = "";
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
    await boxes.register(mailboxAddress.value, mailboxName.value);
    emit("registered", "Mailbox registered. Receiving stays off until the provider route is confirmed.");
    open.value = false;
  } catch (error) {
    notice.value = errorMessage(error);
    pending.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 grid place-items-center bg-ink/40 px-4 py-8" @click.self="!pending && (open = false)">
      <div
        ref="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="register-mailbox-title"
        class="km-card w-full max-w-md p-6 text-ink"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="register-mailbox-title" class="text-xl font-semibold tracking-[-0.03em]">Register a mailbox</h2>
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
          The address is stored on this desk. Receiving stays off until Resend lists that inbox or a message arrives for it.
        </p>
        <form class="mt-5 space-y-3" @submit.prevent="submit">
          <input
            ref="nameInput"
            v-model="mailboxName"
            required
            maxlength="80"
            placeholder="Display name"
            :class="[field, 'bg-canvas']"
            aria-label="Mailbox display name"
          />
          <input
            v-model="mailboxAddress"
            type="email"
            required
            :placeholder="brand.addressHint"
            :class="[field, 'bg-canvas']"
            aria-label="Mailbox address"
          />
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
              {{ pending ? "Registering…" : "Register mailbox" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
