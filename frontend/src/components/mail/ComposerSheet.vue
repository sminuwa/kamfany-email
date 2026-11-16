<script setup lang="ts">
import { Paperclip, X } from "@lucide/vue";
import RichEditor from "../compose/RichEditor.vue";
import { useComposer } from "../../composables/useComposer";
import { formatFileSize, scanNote } from "../../utils/format-mail";

const composer = useComposer();
const fieldClass =
  "mt-1 w-full rounded-control border border-line bg-canvas px-3 text-sm text-ink";

function onFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = input.files ? Array.from(input.files) : [];
  input.value = "";
  void composer.attach(files);
}
</script>

<template>
  <section class="desk-chrome flex h-full min-h-0 flex-1 flex-col bg-surface" aria-labelledby="composer-title">
    <header class="flex h-14 items-center justify-between border-b border-line px-4">
      <div>
        <h2 id="composer-title" class="text-lg font-semibold tracking-[-0.02em]">{{ composer.title }}</h2>
        <p class="text-xs text-muted">{{ composer.saving ? "Saving draft" : composer.status }}</p>
      </div>
      <button
        type="button"
        class="grid size-10 place-items-center rounded-control text-ink hover:bg-canvas"
        aria-label="Close composer"
        @click="composer.close()"
      >
        <X class="size-4" />
      </button>
    </header>
    <form class="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4" @submit.prevent="composer.send()">
      <p class="max-w-[52ch] text-sm leading-relaxed text-muted">{{ composer.note }}</p>
      <p v-if="composer.failure" class="mt-3 text-sm text-danger" role="alert">{{ composer.failure }}</p>
      <label class="mt-4 block text-sm font-medium">
        From
        <select v-model="composer.mailboxId" class="h-10" :class="fieldClass" :disabled="Boolean(composer.draftId)">
          <option v-for="mailbox in composer.senders" :key="mailbox.id" :value="mailbox.id">
            {{ mailbox.displayName }} · {{ mailbox.address }}
          </option>
        </select>
      </label>
      <label class="relative mt-3 block text-sm font-medium">
        To
        <input
          v-model="composer.to"
          required
          class="h-10"
          :class="fieldClass"
          autocomplete="off"
          @input="composer.lookup('to', composer.to)"
        />
      </label>
      <div v-if="composer.suggestions.length && composer.suggestTarget === 'to'" class="mt-1 rounded-control border border-line bg-canvas">
        <button
          v-for="person in composer.suggestions"
          :key="person.address"
          type="button"
          class="block w-full px-3 py-2 text-left text-sm hover:bg-surface"
          @click="composer.chooseSuggestion(person.address)"
        >
          {{ person.name || person.address }}
          <span v-if="person.name" class="text-muted">{{ person.address }}</span>
        </button>
      </div>
      <label class="relative mt-3 block text-sm font-medium">
        Cc
        <input v-model="composer.cc" class="h-10" :class="fieldClass" autocomplete="off" @input="composer.lookup('cc', composer.cc)" />
      </label>
      <div v-if="composer.suggestions.length && composer.suggestTarget === 'cc'" class="mt-1 rounded-control border border-line bg-canvas">
        <button
          v-for="person in composer.suggestions"
          :key="person.address"
          type="button"
          class="block w-full px-3 py-2 text-left text-sm hover:bg-surface"
          @click="composer.chooseSuggestion(person.address)"
        >
          {{ person.name || person.address }}
        </button>
      </div>
      <button v-if="!composer.showBcc" type="button" class="mt-3 self-start text-sm font-medium text-muted hover:text-ink" @click="composer.showBcc = true">
        Add Bcc
      </button>
      <label v-else class="mt-3 block text-sm font-medium">
        Bcc
        <input v-model="composer.bcc" class="h-10" :class="fieldClass" autocomplete="off" @input="composer.lookup('bcc', composer.bcc)" />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Subject
        <input v-model="composer.subject" required class="h-10" :class="fieldClass" />
      </label>
      <div class="mt-3">
        <RichEditor :html="composer.html" :reset-key="composer.resetKey" @change="composer.html = $event" />
      </div>
      <ul v-if="composer.files.length" class="mt-3 flex flex-wrap gap-2">
        <li v-for="file in composer.files" :key="file.id" class="flex items-center gap-2 rounded-control border border-line px-2.5 py-1.5 text-sm">
          <Paperclip class="size-3.5 text-muted" />
          <span>{{ file.filename }}</span>
          <span class="text-muted">{{ formatFileSize(file.sizeBytes) }}</span>
          <span v-if="scanNote(file.scanStatus)" class="text-muted">{{ scanNote(file.scanStatus) }}</span>
          <button type="button" class="text-muted hover:text-ink" :aria-label="`Remove ${file.filename}`" @click="composer.detach(file.id)">
            Remove
          </button>
        </li>
      </ul>
      <div class="mt-4 grid gap-3 border-t border-line pt-4 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <label class="block text-sm font-medium">
          Signature
          <select v-model="composer.signatureId" class="h-10" :class="fieldClass">
            <option value="">None</option>
            <option v-for="item in composer.signatures" :key="item.id" :value="item.id">{{ item.name }}</option>
          </select>
        </label>
        <p v-if="composer.signatureText" class="self-end text-sm leading-relaxed text-muted">{{ composer.signatureText }}</p>
        <label class="block text-sm font-medium">
          New signature name
          <input v-model="composer.signatureName" class="h-10" :class="fieldClass" />
        </label>
        <label class="block text-sm font-medium">
          Signature text
          <textarea v-model="composer.signatureBody" rows="2" class="py-2" :class="fieldClass" />
        </label>
      </div>
      <div class="mt-2 flex flex-wrap gap-2">
        <button type="button" class="text-sm font-medium text-muted hover:text-ink" @click="composer.saveNamedSignature()">Save signature</button>
        <button
          v-for="item in composer.signatures"
          :key="item.id"
          type="button"
          class="text-sm text-muted hover:text-danger"
          @click="composer.removeSignature(item.id)"
        >
          Delete {{ item.name }}
        </button>
      </div>
      <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
        <label class="inline-flex h-10 cursor-pointer items-center gap-2 rounded-full border border-line px-4 text-sm font-medium hover:bg-canvas">
          <Paperclip class="size-4" />
          Attach
          <input class="sr-only" type="file" multiple @change="onFiles" />
        </label>
        <div class="flex gap-2">
          <button type="button" class="h-10 rounded-full px-4 text-sm font-medium text-muted hover:bg-canvas" @click="composer.keep()">
            Save draft
          </button>
          <button type="button" class="h-10 rounded-full px-4 text-sm font-medium text-muted hover:bg-canvas" @click="composer.discard()">
            Discard
          </button>
          <button
            type="submit"
            class="h-10 rounded-full bg-lagoon-ink px-5 text-sm font-semibold text-[#10211f] hover:bg-brass hover:text-brass-ink disabled:opacity-60"
            :disabled="composer.sending"
          >
            {{ composer.sending ? "Sending" : "Send" }}
          </button>
        </div>
      </div>
    </form>
  </section>
</template>
