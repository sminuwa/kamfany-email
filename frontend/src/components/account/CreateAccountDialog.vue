<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { nextTick, ref, watch } from "vue";
import { field } from "../../pages/auth/account-form";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";

const open = defineModel<boolean>({ required: true });
const emit = defineEmits<{ created: [email: string] }>();

const auth = useAuthStore();
const dialog = ref<HTMLElement | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);
const personEmail = ref("");
const personName = ref("");
const personPassword = ref("");
const personRole = ref<"member" | "administrator">("member");
const notice = ref("");
const pending = ref(false);

watch(open, async (isOpen) => {
  notice.value = "";
  pending.value = false;
  if (!isOpen) {
    personEmail.value = "";
    personName.value = "";
    personPassword.value = "";
    personRole.value = "member";
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
  const email = personEmail.value;
  try {
    await auth.addPerson({
      email,
      displayName: personName.value,
      password: personPassword.value,
      role: personRole.value,
    });
    emit("created", email);
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
        aria-labelledby="create-account-title"
        class="km-card w-full max-w-md p-6 text-ink"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="create-account-title" class="text-xl font-semibold tracking-[-0.03em]">Register a mailbox</h2>
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
          An owner or an administrator registers a mailbox for a member or an administrator. The owner role stays with
          the first account. This address is how that person signs in, and nobody else can open the mailbox.
        </p>
        <form class="mt-5 space-y-3" @submit.prevent="submit">
          <input
            ref="nameInput"
            v-model="personName"
            required
            maxlength="80"
            placeholder="Display name"
            :class="[field, 'bg-canvas']"
            aria-label="Mailbox name"
          />
          <input
            v-model="personEmail"
            type="email"
            required
            placeholder="name@kamfany.com"
            :class="[field, 'bg-canvas']"
            aria-label="Mailbox address"
          />
          <input
            v-model="personPassword"
            type="password"
            required
            minlength="12"
            maxlength="128"
            autocomplete="new-password"
            placeholder="Temporary password"
            :class="[field, 'bg-canvas']"
            aria-label="Temporary password"
          />
          <select v-model="personRole" :class="[field, 'bg-canvas']" aria-label="Role">
            <option value="member">Member</option>
            <option value="administrator">Administrator</option>
          </select>
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
