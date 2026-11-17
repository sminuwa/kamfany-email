<script setup lang="ts">
import { Plus, X } from "@lucide/vue";
import { onKeyStroke } from "@vueuse/core";
import { computed, onMounted, ref } from "vue";
import GroupDialog from "../../components/account/GroupDialog.vue";
import GroupImportDialog from "../../components/account/GroupImportDialog.vue";
import { field } from "./account-form";
import type { PersonAccount } from "../../services/auth.service";
import { deleteGroup, fetchGroups, type MailGroup } from "../../services/groups.service";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";

const auth = useAuthStore();
const groups = ref<MailGroup[]>([]);
const people = ref<PersonAccount[]>([]);
const query = ref("");
const notice = ref("");
const failure = ref("");
const createOpen = ref(false);
const editing = ref<MailGroup | null>(null);
const editOpen = ref(false);
const importing = ref<MailGroup | null>(null);
const importOpen = ref(false);
const deleting = ref<MailGroup | null>(null);
const deletePending = ref(false);

const visible = computed(() => {
  const needle = query.value.trim().toLowerCase();
  const rows = [...groups.value].sort((left, right) => left.name.localeCompare(right.name));
  if (!needle) {
    return rows;
  }
  return rows.filter(
    (group) =>
      group.name.toLowerCase().includes(needle) ||
      group.mailboxes.some(
        (mailbox) =>
          mailbox.address.toLowerCase().includes(needle) || mailbox.displayName.toLowerCase().includes(needle),
      ),
  );
});

onMounted(async () => {
  try {
    const [loadedGroups, loadedPeople] = await Promise.all([fetchGroups(), auth.people()]);
    groups.value = loadedGroups;
    people.value = loadedPeople;
  } catch (error) {
    failure.value = errorMessage(error);
  }
});

onKeyStroke("Escape", () => {
  if (deleting.value && !deletePending.value) {
    deleting.value = null;
  }
});

function replace(saved: MailGroup) {
  const exists = groups.value.some((group) => group.id === saved.id);
  groups.value = (exists ? groups.value.map((group) => (group.id === saved.id ? saved : group)) : [...groups.value, saved]).sort(
    (left, right) => left.name.localeCompare(right.name),
  );
  if (editing.value?.id === saved.id) {
    editing.value = saved;
  }
}

function onSaved(saved: MailGroup, message: string) {
  replace(saved);
  notice.value = message;
  failure.value = "";
}

function openEdit(group: MailGroup) {
  editing.value = group;
  editOpen.value = true;
}

function openImport(group: MailGroup) {
  importing.value = group;
  importOpen.value = true;
}

function mailboxLabel(group: MailGroup): string {
  if (group.mailboxes.length === 0) {
    return "No mailboxes yet";
  }
  const shown = group.mailboxes.slice(0, 2).map((mailbox) => mailbox.address);
  const extra = group.mailboxes.length - shown.length;
  return extra > 0 ? `${shown.join(", ")} +${extra}` : shown.join(", ");
}

async function confirmDelete() {
  const group = deleting.value;
  if (!group || deletePending.value) {
    return;
  }
  deletePending.value = true;
  failure.value = "";
  try {
    await deleteGroup(group.id);
    groups.value = groups.value.filter((item) => item.id !== group.id);
    deleting.value = null;
    notice.value = "Group removed.";
  } catch (error) {
    failure.value = errorMessage(error);
  } finally {
    deletePending.value = false;
  }
}
</script>

<template>
  <section>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Groups</h1>
        <p class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
          A group is a named list of mailboxes. Create it first, then add mailboxes one at a time or from a CSV or
          Excel file. Mail sent to the group is delivered to each mailbox in it.
        </p>
      </div>
      <button type="button" class="km-action" @click="createOpen = true">
        <Plus class="size-4" />
        New group
      </button>
    </div>

    <GroupDialog v-model="createOpen" :group="null" :people="people" @saved="onSaved" />
    <GroupDialog v-model="editOpen" :group="editing" :people="people" @saved="onSaved" />
    <GroupImportDialog v-model="importOpen" :group="importing" @imported="onSaved($event.group, '')" />

    <label class="mt-4 block text-sm">
      <span class="sr-only">Search groups</span>
      <input v-model="query" type="search" placeholder="Search groups or mailboxes" :class="field" />
    </label>

    <p v-if="failure" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed text-danger" role="alert">
      {{ failure }}
    </p>
    <p v-else-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <div v-if="visible.length" class="km-card mt-4 overflow-x-auto">
      <table class="km-table">
        <thead>
          <tr>
            <th>Group</th>
            <th>Mailboxes</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="group in visible" :key="group.id">
            <td class="font-medium">{{ group.name }}</td>
            <td>
              <p class="tabular-nums">{{ group.mailboxes.length }}</p>
              <p class="max-w-xs truncate text-muted">{{ mailboxLabel(group) }}</p>
            </td>
            <td>
              <div class="flex flex-wrap gap-1">
                <button type="button" class="km-mini line" @click="openEdit(group)">Edit</button>
                <button type="button" class="km-mini line" @click="openImport(group)">Import</button>
                <button type="button" class="km-mini rose" @click="deleting = group">Delete</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else class="mt-6 text-sm text-muted">
      {{ query.trim() ? "Nothing matches that search." : "No groups yet." }}
    </p>

    <Teleport to="body">
      <div
        v-if="deleting"
        class="fixed inset-0 z-50 grid place-items-center bg-ink/40 px-4 py-8"
        @click.self="!deletePending && (deleting = null)"
      >
        <div role="dialog" aria-modal="true" aria-labelledby="delete-group-title" class="km-card w-full max-w-md p-6 text-ink">
          <div class="flex items-start justify-between gap-4">
            <h2 id="delete-group-title" class="text-xl font-semibold tracking-[-0.03em]">Delete group</h2>
            <button
              type="button"
              class="grid size-8 shrink-0 place-items-center rounded-control text-muted hover:bg-canvas hover:text-ink"
              aria-label="Close"
              :disabled="deletePending"
              @click="deleting = null"
            >
              <X class="size-4" />
            </button>
          </div>
          <p class="mt-2 text-sm leading-relaxed text-muted">
            Delete <span class="font-medium text-ink">{{ deleting.name }}</span>? The mailboxes stay. Only the group is
            removed.
          </p>
          <div class="mt-5 flex items-center justify-end gap-2">
            <button
              type="button"
              class="inline-flex h-10 items-center rounded-full px-4 text-sm font-semibold text-muted hover:bg-canvas hover:text-ink"
              :disabled="deletePending"
              @click="deleting = null"
            >
              Cancel
            </button>
            <button type="button" class="km-mini rose" :disabled="deletePending" @click="confirmDelete">
              {{ deletePending ? "Deleting…" : "Delete group" }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </section>
</template>
