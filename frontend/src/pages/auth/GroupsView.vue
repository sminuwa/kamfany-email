<script setup lang="ts">
import { onMounted, ref } from "vue";
import { errorMessage } from "../../services/http";
import {
  addGroupMailbox,
  createGroup,
  deleteGroup,
  fetchGroups,
  importGroupFile,
  removeGroupMailbox,
  updateGroup,
  type GroupImport,
  type MailGroup,
} from "../../services/groups.service";
import { useAuthStore } from "../../stores/auth";
import type { PersonAccount } from "../../services/auth.service";
import { field } from "./account-form";

const auth = useAuthStore();
const groups = ref<MailGroup[]>([]);
const people = ref<PersonAccount[]>([]);
const notice = ref("");
const name = ref("");
const pending = ref(false);
const renamingId = ref<string | null>(null);
const rename = ref("");
const picks = ref<Record<string, string>>({});

onMounted(async () => {
  try {
    const [loadedGroups, loadedPeople] = await Promise.all([fetchGroups(), auth.people()]);
    groups.value = loadedGroups;
    people.value = loadedPeople;
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

function chooseMailbox(groupId: string, event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  picks.value = { ...picks.value, [groupId]: value };
}

function available(group: MailGroup): PersonAccount[] {
  const taken = new Set(group.mailboxes.map((mailbox) => mailbox.id));
  return people.value.filter((person) => !taken.has(person.mailboxId));
}

function replace(saved: MailGroup) {
  groups.value = groups.value.map((group) => (group.id === saved.id ? saved : group));
}

function startRename(group: MailGroup) {
  renamingId.value = group.id;
  rename.value = group.name;
}

async function saveName(group: MailGroup) {
  notice.value = "";
  try {
    replace(await updateGroup(group.id, { name: rename.value }));
    renamingId.value = null;
    notice.value = "Group renamed.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function create() {
  notice.value = "";
  pending.value = true;
  try {
    const saved = await createGroup(name.value);
    groups.value = [...groups.value, saved].sort((left, right) => left.name.localeCompare(right.name));
    name.value = "";
    notice.value = "Group created. Add mailboxes below.";
  } catch (error) {
    notice.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}

async function addMailbox(group: MailGroup) {
  const mailboxId = picks.value[group.id];
  if (!mailboxId) {
    notice.value = "Choose a mailbox to add.";
    return;
  }
  notice.value = "";
  try {
    replace(await addGroupMailbox(group.id, mailboxId));
    picks.value = { ...picks.value, [group.id]: "" };
    notice.value = "Mailbox added.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function removeMailbox(group: MailGroup, mailboxId: string) {
  notice.value = "";
  try {
    replace(await removeGroupMailbox(group.id, mailboxId));
    notice.value = "Mailbox removed.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

function importSummary(result: GroupImport): string {
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

async function onImport(group: MailGroup, event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  input.value = "";
  if (!file) {
    return;
  }
  notice.value = "";
  try {
    const result = await importGroupFile(group.id, file);
    replace(result.group);
    notice.value = importSummary(result);
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function remove(group: MailGroup) {
  notice.value = "";
  try {
    await deleteGroup(group.id);
    groups.value = groups.value.filter((item) => item.id !== group.id);
    if (renamingId.value === group.id) {
      renamingId.value = null;
    }
    notice.value = "Group removed.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}
</script>

<template>
  <section>
    <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Groups</h1>
    <p class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
      Create a group first. Add mailboxes afterward, one at a time or from a CSV or Excel file. The file should
      list mailbox addresses. Mail sent to the group is delivered to each mailbox in it.
    </p>
    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <form class="km-card mt-4 space-y-3 p-4" @submit.prevent="create">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">New group</h2>
      <label class="block text-sm font-medium">
        Name
        <input v-model="name" required maxlength="80" :class="[field, 'mt-1.5']" />
      </label>
      <button type="submit" class="km-action" :disabled="pending">Create group</button>
    </form>

    <ul v-if="groups.length" class="mt-4 space-y-3">
      <li v-for="group in groups" :key="group.id" class="km-card space-y-3 p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <form v-if="renamingId === group.id" class="flex flex-wrap items-center gap-2" @submit.prevent="saveName(group)">
              <input v-model="rename" required maxlength="80" :class="[field, 'max-w-xs']" aria-label="Group name" />
              <button type="submit" class="km-mini line">Save</button>
              <button type="button" class="km-quiet" @click="renamingId = null">Cancel</button>
            </form>
            <template v-else>
              <h2 class="text-lg font-semibold tracking-[-0.02em]">{{ group.name }}</h2>
              <p class="text-sm text-muted">
                {{ group.mailboxes.length }} {{ group.mailboxes.length === 1 ? "mailbox" : "mailboxes" }}
              </p>
            </template>
          </div>
          <div class="flex gap-2">
            <button v-if="renamingId !== group.id" type="button" class="km-mini line" @click="startRename(group)">
              Rename
            </button>
            <button type="button" class="km-mini rose" @click="remove(group)">Remove</button>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <label class="min-w-0 flex-1 text-sm font-medium">
            <span class="sr-only">Add a mailbox to {{ group.name }}</span>
            <select :value="picks[group.id] ?? ''" :class="field" @change="chooseMailbox(group.id, $event)">
              <option value="">Add a mailbox</option>
              <option v-for="person in available(group)" :key="person.mailboxId" :value="person.mailboxId">
                {{ person.displayName }} — {{ person.email }}
              </option>
            </select>
          </label>
          <button type="button" class="km-action" @click="addMailbox(group)">Add</button>
          <label class="km-quiet cursor-pointer">
            Import CSV or Excel
            <input
              class="sr-only"
              type="file"
              accept=".csv,.xlsx,.xls,text/csv,application/vnd.ms-excel,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
              @change="onImport(group, $event)"
            />
          </label>
        </div>

        <ul class="space-y-1">
          <li v-for="mailbox in group.mailboxes" :key="mailbox.id" class="flex items-center justify-between gap-3 text-sm">
            <span class="min-w-0 truncate">
              {{ mailbox.displayName }}
              <span class="text-muted">{{ mailbox.address }}</span>
            </span>
            <button type="button" class="text-muted hover:text-ink" @click="removeMailbox(group, mailbox.id)">
              Remove
            </button>
          </li>
          <li v-if="group.mailboxes.length === 0" class="text-sm text-muted">No mailboxes yet.</li>
        </ul>
      </li>
    </ul>
    <p v-else class="mt-4 text-sm text-muted">No groups yet.</p>
  </section>
</template>
