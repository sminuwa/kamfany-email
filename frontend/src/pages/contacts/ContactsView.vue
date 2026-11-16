<script setup lang="ts">
import { useContacts } from "../../composables/useContacts";
import { avatarUrl, type Contact } from "../../services/contact.service";

const contacts = useContacts();
const field = "mt-1 h-10 w-full rounded-control border border-line bg-canvas px-3 text-sm text-ink";

function portrait(): string {
  if (!contacts.selectedId || !contacts.hasAvatar) {
    return "";
  }
  return avatarUrl({
    id: contacts.selectedId,
    updatedAt: contacts.updatedAt,
    fullName: contacts.draft.fullName,
    email: contacts.draft.email,
    phone: contacts.draft.phone,
    company: contacts.draft.company,
    notes: contacts.draft.notes,
    hasAvatar: true,
  });
}

function onImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  input.value = "";
  void contacts.importFile(file);
}

function onAvatar(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  input.value = "";
  void contacts.setAvatar(file);
}
</script>

<template>
  <div class="grid h-full min-h-0 grid-cols-1 min-[960px]:grid-cols-[minmax(280px,0.9fr)_minmax(0,1.1fr)]">
    <section class="desk-chrome flex h-full min-h-0 flex-col border-line bg-surface min-[960px]:border-r" aria-label="Contacts">
      <header class="border-b border-line px-4 py-3">
        <h1 class="text-lg font-semibold tracking-[-0.02em]">Contacts</h1>
        <p class="text-xs text-muted">{{ contacts.people.length === 1 ? "1 person" : `${contacts.people.length} people` }}</p>
        <label class="mt-3 block text-sm font-medium">
          Search contacts
          <input v-model="contacts.query" class="h-10" :class="field" type="search" />
        </label>
        <div class="mt-3 flex gap-2">
          <button type="button" class="h-9 rounded-full bg-lagoon-ink px-4 text-sm font-semibold text-[#10211f] hover:bg-brass hover:text-brass-ink" @click="contacts.startNew()">
            New contact
          </button>
          <label class="inline-flex h-9 cursor-pointer items-center rounded-full border border-line px-4 text-sm font-medium hover:bg-canvas">
            Import CSV
            <input class="sr-only" type="file" accept=".csv,text/csv" @change="onImport" />
          </label>
        </div>
      </header>
      <ul class="min-h-0 flex-1 overflow-y-auto">
        <li v-for="person in contacts.people" :key="person.id">
          <button
            type="button"
            class="flex w-full items-center gap-3 border-b border-line px-4 py-3 text-left hover:bg-canvas"
            :class="contacts.selectedId === person.id ? 'bg-canvas' : ''"
            @click="contacts.choose(person as Contact)"
          >
            <span class="grid size-9 shrink-0 place-items-center overflow-hidden rounded-full bg-lagoon text-xs font-semibold text-brass">
              <img v-if="person.hasAvatar" :src="avatarUrl(person)" alt="" class="size-full object-cover" />
              <template v-else>{{ person.fullName.slice(0, 1) }}</template>
            </span>
            <span class="min-w-0">
              <span class="block truncate text-sm font-semibold">{{ person.fullName }}</span>
              <span class="block truncate text-sm text-muted">{{ person.email || person.company || "No email" }}</span>
            </span>
          </button>
        </li>
        <li v-if="contacts.people.length === 0" class="px-4 py-10 text-center text-sm text-muted">
          No contacts yet.
        </li>
      </ul>
    </section>
    <form class="flex h-full min-h-0 flex-col overflow-y-auto bg-canvas px-4 py-4" @submit.prevent="contacts.save()">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">{{ contacts.selectedId ? "Edit contact" : "New contact" }}</h2>
      <p class="mt-1 max-w-[48ch] text-sm leading-relaxed text-muted">
        Contacts belong to you. CSV columns are name, email, phone, company, and notes.
      </p>
      <p v-if="contacts.failure" class="mt-3 text-sm text-danger" role="alert">{{ contacts.failure }}</p>
      <p v-else-if="contacts.notice" class="mt-3 text-sm text-muted">{{ contacts.notice }}</p>
      <div v-if="contacts.selectedId" class="mt-4 flex items-center gap-3">
        <span class="grid size-14 place-items-center overflow-hidden rounded-full bg-lagoon text-sm font-semibold text-brass">
          <img v-if="portrait()" :src="portrait()" alt="" class="size-full object-cover" />
          <template v-else>{{ contacts.draft.fullName.slice(0, 1) || "K" }}</template>
        </span>
        <label class="inline-flex h-9 cursor-pointer items-center rounded-full border border-line bg-surface px-4 text-sm font-medium hover:bg-canvas">
          Profile image
          <input class="sr-only" type="file" accept="image/png,image/jpeg,image/gif,image/webp" @change="onAvatar" />
        </label>
        <button v-if="contacts.hasAvatar" type="button" class="text-sm text-muted hover:text-ink" @click="contacts.clearAvatar()">
          Remove image
        </button>
      </div>
      <label class="mt-4 block text-sm font-medium">
        Full name
        <input v-model="contacts.draft.fullName" required maxlength="120" :class="field" />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Email
        <input v-model="contacts.draft.email" type="email" :class="field" />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Phone
        <input v-model="contacts.draft.phone" maxlength="40" :class="field" />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Company
        <input v-model="contacts.draft.company" maxlength="120" :class="field" />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Notes
        <textarea v-model="contacts.draft.notes" rows="4" maxlength="4000" class="mt-1 w-full rounded-control border border-line bg-surface px-3 py-2 text-sm text-ink" />
      </label>
      <div class="mt-4 flex gap-2">
        <button type="submit" class="h-10 rounded-full bg-lagoon-ink px-5 text-sm font-semibold text-[#10211f] hover:bg-brass hover:text-brass-ink disabled:opacity-60" :disabled="contacts.saving">
          {{ contacts.saving ? "Saving" : "Save contact" }}
        </button>
        <button v-if="contacts.selectedId" type="button" class="h-10 rounded-full px-4 text-sm font-medium text-muted hover:bg-surface" @click="contacts.remove()">
          Delete
        </button>
      </div>
    </form>
  </div>
</template>
