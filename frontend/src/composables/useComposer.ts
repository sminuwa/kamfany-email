import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";
import {
  createDraft,
  createSignature,
  deleteDraft,
  deleteDraftFile,
  deleteSignature,
  fetchDraft,
  fetchSignatures,
  saveDraft,
  suggestRecipients,
  uploadDraftFile,
  type DraftFile,
  type MailSignature,
  type RecipientSuggestion,
} from "../services/compose.service";
import { splitAddresses } from "../services/message.service";
import { useMailboxStore } from "../stores/mailboxes";
import { useMailStore } from "../stores/messages";
import { useUiStore } from "../stores/ui";
import type { ComposeKind } from "../types/mail";
import { prepareCompose } from "../utils/compose";
import { htmlToText, textToHtml, withSignature } from "../utils/message-html";

export function useComposer() {
  const ui = useUiStore();
  const mail = useMailStore();
  const boxes = useMailboxStore();
  const mailboxId = ref("");
  const to = ref("");
  const cc = ref("");
  const bcc = ref("");
  const showCc = ref(false);
  const showBcc = ref(false);
  const subject = ref("");
  const html = ref("<p></p>");
  const resetKey = ref("0");
  const draftId = ref<string | null>(null);
  const kind = ref<ComposeKind>("new");
  const messageId = ref<string | null>(null);
  const files = ref<DraftFile[]>([]);
  const signatures = ref<MailSignature[]>([]);
  const signatureId = ref("");
  const suggestions = ref<RecipientSuggestion[]>([]);
  const suggestTarget = ref<"to" | "cc" | "bcc" | null>(null);
  const failure = ref("");
  const status = ref("");
  const sending = ref(false);
  const saving = ref(false);
  const signatureName = ref("");
  const signatureBody = ref("");
  let timer = 0;
  let skipSave = false;

  const senders = computed(() => boxes.items.filter((mailbox) => mailbox.canSend && mailbox.isEnabled));
  const title = computed(() => {
    const message = sourceMessage();
    return prepareCompose(kind.value, message, mail.address).title;
  });
  const note = computed(() => {
    const message = sourceMessage();
    return prepareCompose(kind.value, message, mail.address).note;
  });
  const signature = computed(() => signatures.value.find((item) => item.id === signatureId.value) ?? null);
  const signatureText = computed(() => htmlToText(signature.value?.html ?? ""));

  function sourceMessage() {
    const id = messageId.value;
    if (!id) {
      return null;
    }
    return mail.detail?.id === id ? mail.detail : (mail.byId(id) ?? null);
  }

  function fields() {
    return {
      kind: kind.value,
      inReplyTo: kind.value === "new" ? null : messageId.value,
      to: splitAddresses(to.value),
      cc: splitAddresses(cc.value),
      bcc: splitAddresses(bcc.value),
      subject: subject.value,
      html: html.value,
    };
  }

  function dirty(): boolean {
    return (
      to.value.trim() !== "" ||
      cc.value.trim() !== "" ||
      bcc.value.trim() !== "" ||
      subject.value.trim() !== "" ||
      htmlToText(html.value) !== "" ||
      files.value.length > 0
    );
  }

  async function loadSignatures() {
    if (!mailboxId.value) {
      signatures.value = [];
      return;
    }
    signatures.value = await fetchSignatures(mailboxId.value);
    const preferred = signatures.value.find((item) => item.isDefault);
    signatureId.value = preferred?.id ?? "";
  }

  async function applyIntent() {
    window.clearTimeout(timer);
    skipSave = true;
    failure.value = "";
    status.value = "";
    suggestions.value = [];
    const intent = ui.composeIntent;
    kind.value = intent.kind;
    messageId.value = intent.messageId;
    draftId.value = intent.draftId;
    files.value = [];
    showCc.value = false;
    showBcc.value = false;
    mailboxId.value = mail.mailboxId ?? senders.value[0]?.id ?? "";
    if (intent.draftId) {
      const draft = await fetchDraft(intent.draftId);
      mailboxId.value = draft.mailboxId;
      kind.value = draft.kind;
      messageId.value = draft.inReplyTo;
      to.value = draft.to.join(", ");
      cc.value = draft.cc.join(", ");
      bcc.value = draft.bcc.join(", ");
      showCc.value = draft.cc.length > 0;
      showBcc.value = draft.bcc.length > 0;
      subject.value = draft.subject;
      html.value = draft.html || "<p></p>";
      files.value = draft.attachments;
    } else {
      const prepared = prepareCompose(intent.kind, sourceMessage(), mail.address);
      to.value = prepared.to;
      cc.value = prepared.cc;
      showCc.value = prepared.cc.trim() !== "";
      bcc.value = "";
      subject.value = prepared.subject;
      html.value = textToHtml(prepared.body);
    }
    resetKey.value = `${intent.draftId ?? "new"}-${intent.messageId ?? ""}-${intent.kind}`;
    await loadSignatures();
    skipSave = false;
  }

  function scheduleSave() {
    if (skipSave || !mailboxId.value || !dirty()) {
      return;
    }
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      void persist();
    }, 1200);
  }

  async function persist() {
    if (!mailboxId.value || !dirty()) {
      return;
    }
    saving.value = true;
    try {
      const draft = draftId.value
        ? await saveDraft(draftId.value, fields())
        : await createDraft(mailboxId.value, fields());
      draftId.value = draft.id;
      status.value = "Draft saved";
      failure.value = "";
    } catch (error) {
      failure.value = error instanceof Error ? error.message : "The draft was not saved.";
    } finally {
      saving.value = false;
    }
  }

  async function ensureDraft() {
    if (draftId.value) {
      return draftId.value;
    }
    const draft = await createDraft(mailboxId.value, fields());
    draftId.value = draft.id;
    return draft.id;
  }

  async function attach(list: FileList | readonly File[] | null) {
    const filesToStore = list ? Array.from(list) : [];
    if (filesToStore.length === 0 || !mailboxId.value) {
      return;
    }
    failure.value = "";
    try {
      const id = await ensureDraft();
      for (const file of filesToStore) {
        files.value = [...files.value, await uploadDraftFile(id, file)];
      }
      status.value = "Draft saved";
    } catch (error) {
      failure.value = error instanceof Error ? error.message : "The file was not attached.";
    }
  }

  async function detach(fileId: string) {
    if (!draftId.value) {
      return;
    }
    await deleteDraftFile(draftId.value, fileId);
    files.value = files.value.filter((file) => file.id !== fileId);
  }

  async function lookup(target: "to" | "cc" | "bcc", value: string) {
    suggestTarget.value = target;
    const part = value.split(/[,;]/).at(-1)?.trim() ?? "";
    if (part.length < 1 || !mailboxId.value) {
      suggestions.value = [];
      return;
    }
    suggestions.value = await suggestRecipients(mailboxId.value, part);
  }

  function chooseSuggestion(address: string) {
    const target = suggestTarget.value ?? "to";
    const current = target === "to" ? to : target === "cc" ? cc : bcc;
    const parts = splitAddresses(current.value);
    parts.pop();
    if (!parts.some((item) => item.toLowerCase() === address.toLowerCase())) {
      parts.push(address);
    }
    current.value = parts.join(", ");
    suggestions.value = [];
  }

  async function send() {
    failure.value = "";
    sending.value = true;
    try {
      await persist();
      const outgoing = withSignature(html.value, signature.value?.html ?? "");
      await mail.deliver({
        kind: kind.value,
        mailboxId: mailboxId.value,
        messageId: messageId.value,
        draftId: draftId.value,
        to: to.value,
        cc: cc.value,
        bcc: bcc.value,
        subject: subject.value,
        text: htmlToText(outgoing),
        html: outgoing,
      });
      ui.closeComposer();
    } catch (error) {
      failure.value = error instanceof Error ? error.message : "The message was not sent.";
    } finally {
      sending.value = false;
    }
  }

  async function close() {
    window.clearTimeout(timer);
    if (dirty() && mailboxId.value) {
      await persist();
      if (failure.value) {
        return;
      }
    }
    if (draftId.value) {
      await mail.reload();
    }
    ui.closeComposer();
  }

  async function keep() {
    window.clearTimeout(timer);
    await persist();
    if (!failure.value && draftId.value) {
      await mail.reload();
    }
  }

  async function discard() {
    window.clearTimeout(timer);
    if (draftId.value) {
      await deleteDraft(draftId.value);
      await mail.reload();
    }
    ui.closeComposer();
  }

  async function saveNamedSignature() {
    if (!signatureName.value.trim()) {
      return false;
    }
    const created = await createSignature({
      mailboxId: mailboxId.value || null,
      name: signatureName.value.trim(),
      html: textToHtml(signatureBody.value),
      isDefault: signatures.value.length === 0,
    });
    signatures.value = [...signatures.value, created];
    signatureId.value = created.id;
    signatureName.value = "";
    signatureBody.value = "";
    return true;
  }

  async function removeSignature(id: string) {
    await deleteSignature(id);
    signatures.value = signatures.value.filter((item) => item.id !== id);
    if (signatureId.value === id) {
      signatureId.value = "";
    }
  }

  watch(
    () => [ui.composeIntent.kind, ui.composeIntent.messageId, ui.composeIntent.draftId, ui.composerOpen] as const,
    () => {
      if (ui.composerOpen) {
        void applyIntent().catch((error: unknown) => {
          failure.value = error instanceof Error ? error.message : "The draft could not be opened.";
        });
      }
    },
    { immediate: true },
  );

  watch([to, cc, bcc, subject, html, mailboxId], scheduleSave);

  onBeforeUnmount(() => window.clearTimeout(timer));

  return reactive({
    mailboxId,
    senders,
    to,
    cc,
    bcc,
    showCc,
    showBcc,
    subject,
    html,
    resetKey,
    files,
    signatures,
    signatureId,
    signature,
    signatureText,
    suggestions,
    suggestTarget,
    failure,
    status,
    sending,
    saving,
    signatureName,
    signatureBody,
    title,
    note,
    draftId,
    send,
    close,
    discard,
    attach,
    detach,
    lookup,
    chooseSuggestion,
    saveNamedSignature,
    removeSignature,
    persist,
    keep,
  });
}
