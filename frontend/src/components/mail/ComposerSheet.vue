<script setup lang="ts">
import { Paperclip, X } from "@lucide/vue";
import { ref } from "vue";
import RichEditor from "../compose/RichEditor.vue";
import { useComposer } from "../../composables/useComposer";
import { formatFileSize, scanNote } from "../../utils/format-mail";
import SignatureDialog from "./SignatureDialog.vue";

const composer = useComposer();
const signatureOpen = ref(false);
const fieldClass =
  "mt-1 w-full rounded-control border border-line bg-canvas px-3 text-sm text-ink placeholder:text-muted";

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
      <div class="relative mt-4">
        <label class="block text-sm font-medium" for="compose-to">To</label>
        <input
          id="compose-to"
          v-model="composer.to"
          required
          placeholder="name@kamfany.com"
          class="h-10"
          :class="[fieldClass, composer.showCc && composer.showBcc ? '' : 'pr-24']"
          autocomplete="off"
          @input="composer.lookup('to', composer.to)"
        />
        <div
          v-if="!composer.showCc || !composer.showBcc"
          class="absolute bottom-0 right-3 flex h-10 items-center gap-3"
        >
          <button
            v-if="!composer.showCc"
            type="button"
            class="text-sm font-medium text-muted hover:text-ink"
            @click="composer.showCc = true"
          >
            Cc
          </button>
          <button
            v-if="!composer.showBcc"
            type="button"
            class="text-sm font-medium text-muted hover:text-ink"
            @click="composer.showBcc = true"
          >
            Bcc
          </button>
        </div>
      </div>
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
      <label v-if="composer.showCc" class="relative mt-3 block text-sm font-medium">
        Cc
        <input
          v-model="composer.cc"
          placeholder="name@kamfany.com"
          class="h-10"
          :class="fieldClass"
          autocomplete="off"
          @input="composer.lookup('cc', composer.cc)"
        />
      </label>
      <div v-if="composer.showCc && composer.suggestions.length && composer.suggestTarget === 'cc'" class="mt-1 rounded-control border border-line bg-canvas">
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
      <label v-if="composer.showBcc" class="mt-3 block text-sm font-medium">
        Bcc
        <input
          v-model="composer.bcc"
          placeholder="name@kamfany.com"
          class="h-10"
          :class="fieldClass"
          autocomplete="off"
          @input="composer.lookup('bcc', composer.bcc)"
        />
      </label>
      <label class="mt-3 block text-sm font-medium">
        Subject
        <input v-model="composer.subject" required placeholder="Subject" class="h-10" :class="fieldClass" />
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
      <div class="mt-4 border-t border-line pt-4">
        <div class="flex items-end gap-3">
          <label class="block min-w-0 flex-1 text-sm font-medium">
            Signature
            <select v-model="composer.signatureId" class="h-10" :class="fieldClass">
              <option value="">None</option>
              <option v-for="item in composer.signatures" :key="item.id" :value="item.id">{{ item.name }}</option>
            </select>
          </label>
          <button
            type="button"
            class="h-10 shrink-0 rounded-full px-4 text-sm font-medium text-muted hover:bg-canvas hover:text-ink"
            @click="signatureOpen = true"
          >
            Add signature
          </button>
        </div>
        <p v-if="composer.signatureText" class="mt-2 text-sm leading-relaxed text-muted">{{ composer.signatureText }}</p>
      </div>
      <SignatureDialog
        v-model="signatureOpen"
        v-model:name="composer.signatureName"
        v-model:body="composer.signatureBody"
        :signatures="composer.signatures"
        :save="composer.saveNamedSignature"
        :remove="composer.removeSignature"
      />
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
