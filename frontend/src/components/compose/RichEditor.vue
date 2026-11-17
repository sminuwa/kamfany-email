<script setup lang="ts">
import Color from "@tiptap/extension-color";
import TextAlign from "@tiptap/extension-text-align";
import { TextStyle } from "@tiptap/extension-text-style";
import StarterKit from "@tiptap/starter-kit";
import { EditorContent, useEditor } from "@tiptap/vue-3";
import { computed, watch } from "vue";

const props = defineProps<{
  html: string;
  resetKey: string;
}>();

const emit = defineEmits<{
  change: [html: string];
}>();

const editor = useEditor({
  content: props.html,
  extensions: [
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
      link: { openOnClick: false, autolink: true },
    }),
    TextAlign.configure({ types: ["heading", "paragraph"] }),
    TextStyle,
    Color,
  ],
  editorProps: {
    attributes: {
      class: "composer-prose min-h-48 px-3 py-2 text-[15px] leading-relaxed outline-none",
      "aria-label": "Message",
    },
  },
  onUpdate: ({ editor: current }) => emit("change", current.getHTML()),
});

watch(
  () => props.resetKey,
  () => {
    editor.value?.commands.setContent(props.html || "<p></p>", { emitUpdate: false });
  },
);

const empty = computed(() => props.html.replace(/<[^>]+>/g, "").replace(/&nbsp;/g, " ").trim() === "");

const colors = [
  { name: "Ink", value: "#10211f" },
  { name: "Lagoon", value: "#1f6f68" },
  { name: "Brass", value: "#a9782f" },
];

function run(action: () => boolean) {
  action();
}

function addLink() {
  const current = editor.value;
  if (!current) {
    return;
  }
  const previous = current.getAttributes("link").href as string | undefined;
  const next = window.prompt("Link address", previous ?? "https://");
  if (next === null) {
    return;
  }
  if (next.trim() === "") {
    current.chain().focus().unsetLink().run();
    return;
  }
  current.chain().focus().setLink({ href: next.trim() }).run();
}
</script>

<template>
  <div class="overflow-hidden rounded-control border border-line bg-canvas">
    <div v-if="editor" class="flex flex-wrap items-center gap-1 border-b border-line px-2 py-1.5">
      <button type="button" class="toolbar" :class="editor.isActive('bold') ? 'is-on' : ''" aria-label="Bold" @click="run(() => editor!.chain().focus().toggleBold().run())">B</button>
      <button type="button" class="toolbar italic" :class="editor.isActive('italic') ? 'is-on' : ''" aria-label="Italic" @click="run(() => editor!.chain().focus().toggleItalic().run())">I</button>
      <button type="button" class="toolbar underline" :class="editor.isActive('underline') ? 'is-on' : ''" aria-label="Underline" @click="run(() => editor!.chain().focus().toggleUnderline().run())">U</button>
      <button type="button" class="toolbar" :class="editor.isActive('heading', { level: 2 }) ? 'is-on' : ''" aria-label="Heading" @click="run(() => editor!.chain().focus().toggleHeading({ level: 2 }).run())">H</button>
      <button type="button" class="toolbar" :class="editor.isActive('bulletList') ? 'is-on' : ''" aria-label="Bulleted list" @click="run(() => editor!.chain().focus().toggleBulletList().run())">•</button>
      <button type="button" class="toolbar" :class="editor.isActive('orderedList') ? 'is-on' : ''" aria-label="Numbered list" @click="run(() => editor!.chain().focus().toggleOrderedList().run())">1.</button>
      <button type="button" class="toolbar" :class="editor.isActive('blockquote') ? 'is-on' : ''" aria-label="Quote" @click="run(() => editor!.chain().focus().toggleBlockquote().run())">”</button>
      <button type="button" class="toolbar" aria-label="Align left" @click="run(() => editor!.chain().focus().setTextAlign('left').run())">L</button>
      <button type="button" class="toolbar" aria-label="Align center" @click="run(() => editor!.chain().focus().setTextAlign('center').run())">C</button>
      <button type="button" class="toolbar" aria-label="Align right" @click="run(() => editor!.chain().focus().setTextAlign('right').run())">R</button>
      <button type="button" class="toolbar" :class="editor.isActive('link') ? 'is-on' : ''" aria-label="Link" @click="addLink">Link</button>
      <button
        v-for="color in colors"
        :key="color.value"
        type="button"
        class="grid size-7 place-items-center rounded-full"
        :aria-label="`${color.name} text`"
        @click="run(() => editor!.chain().focus().setColor(color.value).run())"
      >
        <span class="size-3 rounded-full" :style="{ backgroundColor: color.value }" />
      </button>
    </div>
    <div class="relative">
      <p v-if="empty" class="pointer-events-none absolute left-3 top-2 text-[15px] leading-relaxed text-muted">
        Write your message
      </p>
      <EditorContent :editor="editor" />
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  min-width: 1.75rem;
  height: 1.75rem;
  border-radius: 999px;
  padding: 0 0.4rem;
  font-size: 0.8rem;
  font-weight: 650;
}
.toolbar.is-on,
.toolbar:hover {
  background: var(--color-surface);
}
.italic {
  font-style: italic;
}
.underline {
  text-decoration: underline;
}
:deep(.composer-prose p) {
  margin: 0.35rem 0;
}
:deep(.composer-prose ul) {
  margin: 0.35rem 0;
  list-style: disc;
  padding-left: 1.25rem;
}
:deep(.composer-prose ol) {
  margin: 0.35rem 0;
  list-style: decimal;
  padding-left: 1.25rem;
}
:deep(.composer-prose blockquote) {
  margin: 0.5rem 0;
  border-left: 2px solid var(--color-line);
  padding-left: 0.75rem;
  color: var(--color-muted);
}
:deep(.composer-prose a) {
  color: var(--color-lagoon-ink);
  text-decoration: underline;
}
</style>
