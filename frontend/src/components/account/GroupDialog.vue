<script setup lang="ts">
import { X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { computed, nextTick, ref, watch } from "vue";
import { field } from "../../pages/auth/account-form";
import type { PersonAccount } from "../../services/auth.service";
import {
  addGroupMailbox,
  createGroup,
  removeGroupMailbox,
  updateGroup,
  type MailGroup,
} from "../../services/groups.service";
import { errorMessage } from "../../services/http";

const open = defineModel<boolean>({ required: true });
const props = defineProps<{
  group: MailGroup | null;
  people: PersonAccount[];
}>();
const emit = defineEmits<{ saved: [group: MailGroup, message: string] }>();

const dialog = ref<HTMLElement | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);
const current = ref<MailGroup | null>(null);
const name = ref("");
const mailboxId = ref("");
const notice = ref("");
const failure = ref("");
const pending = ref(false);

const editing = computed(() => current.value !== null);
const available = computed(() => {
  const taken = new Set(current.value?.mailboxes.map((mailbox) => mailbox.id) ?? []);
  return props.people.filter((person) => !taken.has(person.mailboxId));
});

watch(open, async (isOpen) => {
  failure.value = "";
  notice.value = "";
  pending.value = false;
  mailboxId.value = "";
  if (!isOpen) {
    name.value = "";
    current.value = null;
    return;
  }
  current.value = props.group
    ? { ...props.group, mailboxes: props.group.mailboxes.map((mailbox) => ({ ...mailbox })) }
    : null;
  name.value = props.group?.name ?? "";
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

function publish(group: MailGroup, message: string) {
  current.value = { ...group, mailboxes: group.mailboxes.map((mailbox) => ({ ...mailbox })) };
  name.value = group.name;
  notice.value = message;
  failure.value = "";
  emit("saved", group, message);
}

async function submitName() {
  if (pending.value) {
    return;
  }
  failure.value = "";
  notice.value = "";
  pending.value = true;
  try {
    if (!current.value) {
      const saved = await createGroup(name.value);
      emit("saved", saved, "Group created.");
      open.value = false;
      return;
    }
    const saved = await updateGroup(current.value.id, { name: name.value });
    publish(saved, "Group renamed.");
  } catch (error) {
    failure.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}

async function addMailbox() {
  if (!current.value || !mailboxId.value || pending.value) {
    failure.value = mailboxId.value ? "" : "Choose a mailbox to add.";
    return;
  }
  pending.value = true;
  failure.value = "";
  try {
    const saved = await addGroupMailbox(current.value.id, mailboxId.value);
    mailboxId.value = "";
    publish(saved, "Mailbox added.");
  } catch (error) {
    failure.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}

async function removeMailbox(mailboxIdToRemove: string) {
  if (!current.value || pending.value) {
    return;
  }
  pending.value = true;
  failure.value = "";
  try {
    const saved = await removeGroupMailbox(current.value.id, mailboxIdToRemove);
    publish(saved, "Mailbox removed.");
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
        aria-labelledby="group-dialog-title"
        class="km-card max-h-[85vh] w-full max-w-lg overflow-y-auto p-6 text-ink"
        @keydown="trapTab"
      >
        <div class="flex items-start justify-between gap-4">
          <h2 id="group-dialog-title" class="text-xl font-semibold tracking-[-0.03em]">
            {{ editing ? "Edit group" : "New group" }}
          </h2>
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
          {{
            editing
              ? "Rename the group, or add and remove mailboxes. Mail sent to the group reaches each mailbox in it."
              : "A group is a name only. Add mailboxes after it is created."
          }}
        </p>
        <form class="mt-5 space-y-3" @submit.prevent="submitName">
          <label class="block text-sm font-medium">
            Name
            <input
              ref="nameInput"
              v-model="name"
              required
              maxlength="80"
              :class="[field, 'mt-1.5 bg-canvas']"
            />
          </label>
          <p v-if="failure" class="text-sm leading-relaxed text-danger" role="alert">{{ failure }}</p>
          <p v-else-if="notice" class="text-sm leading-relaxed text-muted" role="status">{{ notice }}</p>
          <div class="flex items-center justify-end gap-2 pt-1">
            <button
              type="button"
              class="inline-flex h-10 items-center rounded-full px-4 text-sm font-semibold text-muted hover:bg-canvas hover:text-ink"
              :disabled="pending"
              @click="open = false"
            >
              {{ editing ? "Close" : "Cancel" }}
            </button>
            <button type="submit" class="km-action" :disabled="pending">
              {{ editing ? "Save name" : pending ? "Creating…" : "Create group" }}
            </button>
          </div>
        </form>

        <div v-if="current" class="mt-6 border-t border-line pt-5">
          <h3 class="text-sm font-semibold">Mailboxes</h3>
          <div class="mt-3 flex flex-wrap items-center gap-2">
            <label class="min-w-0 flex-1 text-sm font-medium">
              <span class="sr-only">Mailbox to add</span>
              <select v-model="mailboxId" :class="[field, 'bg-canvas']">
                <option value="">Add a mailbox</option>
                <option v-for="person in available" :key="person.mailboxId" :value="person.mailboxId">
                  {{ person.displayName }} — {{ person.email }}{{ person.mailboxEnabled ? "" : " (disabled)" }}
                </option>
              </select>
            </label>
            <button type="button" class="km-action" :disabled="pending" @click="addMailbox">Add</button>
          </div>
          <p v-if="available.length === 0" class="mt-2 text-sm text-muted">Every mailbox is already in this group.</p>
          <ul class="mt-3 space-y-1">
            <li
              v-for="mailbox in current.mailboxes"
              :key="mailbox.id"
              class="flex items-center justify-between gap-3 text-sm"
            >
              <span class="min-w-0 truncate">
                {{ mailbox.displayName }}
                <span class="text-muted">{{ mailbox.address }}</span>
              </span>
              <button
                type="button"
                class="text-muted hover:text-ink"
                :disabled="pending"
                @click="removeMailbox(mailbox.id)"
              >
                Remove
              </button>
            </li>
            <li v-if="current.mailboxes.length === 0" class="text-sm text-muted">No mailboxes yet.</li>
          </ul>
        </div>
      </div>
    </div>
  </Teleport>
</template>
