<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { computed, nextTick, ref, watch } from "vue";
import { field } from "../../pages/auth/account-form";
import { errorMessage } from "../../services/http";
import { useMailboxStore } from "../../stores/mailboxes";

const open = defineModel<boolean>({ required: true });
const props = defineProps<{ mailboxId: string }>();
const emit = defineEmits<{ granted: [message: string] }>();

const boxes = useMailboxStore();
const dialog = ref<HTMLElement | null>(null);
const emailInput = ref<HTMLInputElement | null>(null);
const grantEmail = ref("");
const grantMailboxId = ref("");
const grantSend = ref(false);
const grantManage = ref(false);
const notice = ref("");
const pending = ref(false);
const manageable = computed(() => boxes.items.filter((mailbox) => mailbox.canManage));

watch(open, async (isOpen) => {
  notice.value = "";
  pending.value = false;
  if (!isOpen) {
    grantEmail.value = "";
    grantSend.value = false;
    grantManage.value = false;
    return;
  }
  grantMailboxId.value = manageable.value.some((mailbox) => mailbox.id === props.mailboxId)
    ? props.mailboxId
    : (manageable.value[0]?.id ?? "");
  await nextTick();
  emailInput.value?.focus();
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
  if (pending.value || !grantMailboxId.value) {
    return;
  }
  notice.value = "";
  pending.value = true;
  try {
    await boxes.grant(grantMailboxId.value, {
      email: grantEmail.value,
      canRead: true,
      canSend: grantSend.value,
      canManage: grantManage.value,
    });
    emit("granted", "Access granted.");
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
        aria-labelledby="grant-access-title"
        class="w-full max-w-md rounded-panel border border-line bg-surface p-6 text-ink shadow-panel"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="grant-access-title" class="text-xl font-semibold tracking-[-0.03em]">Grant access</h2>
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
        <p class="mt-2 text-sm leading-relaxed text-muted">The person needs an account first. Send and manage include reading.</p>
        <p v-if="manageable.length === 0" class="mt-5 text-sm text-muted">No mailbox can be shared from this account.</p>
        <form v-else class="mt-5 space-y-3" @submit.prevent="submit">
          <select v-model="grantMailboxId" :class="[field, 'bg-canvas']" aria-label="Mailbox to share">
            <option v-for="mailbox in manageable" :key="mailbox.id" :value="mailbox.id">
              {{ mailbox.address }}
            </option>
          </select>
          <input
            ref="emailInput"
            v-model="grantEmail"
            type="email"
            required
            placeholder="person@kamfany.com"
            :class="[field, 'bg-canvas']"
            aria-label="Person email"
          />
          <label class="flex items-center gap-2 text-sm">
            <input v-model="grantSend" type="checkbox" class="size-4 accent-brass" />
            Can send
          </label>
          <label class="flex items-center gap-2 text-sm">
            <input v-model="grantManage" type="checkbox" class="size-4 accent-brass" />
            Can manage
          </label>
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
              {{ pending ? "Granting…" : "Grant access" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
