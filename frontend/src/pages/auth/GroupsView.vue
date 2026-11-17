<script setup lang="ts">
import { onMounted, ref } from "vue";
import { errorMessage } from "../../services/http";
import {
  createGroup,
  deleteGroup,
  fetchGroups,
  updateGroup,
  type MailGroup,
} from "../../services/groups.service";
import { useAuthStore } from "../../stores/auth";
import { useBrandStore } from "../../stores/brand";
import type { PersonAccount } from "../../services/auth.service";
import { field } from "./account-form";

const auth = useAuthStore();
const brand = useBrandStore();
const groups = ref<MailGroup[]>([]);
const people = ref<PersonAccount[]>([]);
const notice = ref("");
const name = ref("");
const address = ref("");
const selected = ref<string[]>([]);
const editingId = ref<string | null>(null);
const pending = ref(false);

onMounted(async () => {
  try {
    const [loadedGroups, loadedPeople] = await Promise.all([fetchGroups(), auth.people()]);
    groups.value = loadedGroups;
    people.value = loadedPeople;
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

function resetForm() {
  editingId.value = null;
  name.value = "";
  address.value = "";
  selected.value = [];
}

function edit(group: MailGroup) {
  editingId.value = group.id;
  name.value = group.name;
  address.value = group.address.split("@")[0] ?? "";
  selected.value = group.mailboxes.map((mailbox) => mailbox.id);
}

function toggle(mailboxId: string) {
  selected.value = selected.value.includes(mailboxId)
    ? selected.value.filter((id) => id !== mailboxId)
    : [...selected.value, mailboxId];
}

async function save() {
  notice.value = "";
  pending.value = true;
  try {
    if (editingId.value) {
      const saved = await updateGroup(editingId.value, {
        name: name.value,
        mailboxIds: selected.value,
      });
      groups.value = groups.value.map((group) => (group.id === saved.id ? saved : group));
      notice.value = "Group updated.";
    } else {
      const saved = await createGroup({
        name: name.value,
        address: address.value,
        mailboxIds: selected.value,
      });
      groups.value = [...groups.value, saved].sort((left, right) =>
        left.address.localeCompare(right.address),
      );
      notice.value = "Group created.";
    }
    resetForm();
  } catch (error) {
    notice.value = errorMessage(error);
  } finally {
    pending.value = false;
  }
}

async function remove(group: MailGroup) {
  notice.value = "";
  try {
    await deleteGroup(group.id);
    groups.value = groups.value.filter((item) => item.id !== group.id);
    if (editingId.value === group.id) {
      resetForm();
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
      A group is one address. Mail sent to it is delivered to every mailbox in the group.
    </p>
    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <form class="km-card mt-4 space-y-3 p-4" @submit.prevent="save">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">
        {{ editingId ? "Edit group" : "New group" }}
      </h2>
      <label class="block text-sm font-medium">
        Name
        <input v-model="name" required maxlength="80" :class="[field, 'mt-1.5']" />
      </label>
      <label class="block text-sm font-medium">
        Address
        <span class="mt-1.5 flex items-center gap-2">
          <input
            v-model="address"
            required
            maxlength="64"
            :disabled="Boolean(editingId)"
            :class="[field, 'disabled:opacity-60']"
            placeholder="support"
            autocapitalize="off"
            autocomplete="off"
          />
          <span class="shrink-0 text-sm text-muted">@{{ brand.domain || "domain" }}</span>
        </span>
      </label>
      <fieldset>
        <legend class="text-sm font-medium">Mailboxes</legend>
        <p v-if="people.length === 0" class="mt-2 text-sm text-muted">No mailboxes are available.</p>
        <ul v-else class="mt-2 max-h-64 space-y-1 overflow-y-auto">
          <li v-for="person in people" :key="person.mailboxId">
            <label class="flex items-center gap-2 rounded-control px-2 py-1.5 text-sm hover:bg-canvas">
              <input
                type="checkbox"
                :checked="selected.includes(person.mailboxId)"
                @change="toggle(person.mailboxId)"
              />
              <span class="min-w-0 truncate">
                {{ person.displayName }}
                <span class="text-muted">{{ person.email }}</span>
              </span>
            </label>
          </li>
        </ul>
      </fieldset>
      <div class="flex items-center gap-2">
        <button type="submit" class="km-action" :disabled="pending">
          {{ editingId ? "Save group" : "Create group" }}
        </button>
        <button v-if="editingId" type="button" class="km-quiet" @click="resetForm">Cancel</button>
      </div>
    </form>

    <ul v-if="groups.length" class="mt-4 space-y-3">
      <li v-for="group in groups" :key="group.id" class="km-card p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold tracking-[-0.02em]">{{ group.name }}</h2>
            <p class="text-sm text-muted">{{ group.address }}</p>
          </div>
          <div class="flex gap-2">
            <button type="button" class="km-mini line" @click="edit(group)">Edit</button>
            <button type="button" class="km-mini rose" @click="remove(group)">Remove</button>
          </div>
        </div>
        <ul class="mt-3 space-y-1">
          <li v-for="mailbox in group.mailboxes" :key="mailbox.id" class="text-sm">
            {{ mailbox.displayName }}
            <span class="text-muted">{{ mailbox.address }}</span>
          </li>
          <li v-if="group.mailboxes.length === 0" class="text-sm text-muted">No mailboxes yet.</li>
        </ul>
      </li>
    </ul>
    <p v-else class="mt-4 text-sm text-muted">No groups yet.</p>
  </section>
</template>
