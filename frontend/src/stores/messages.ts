import { defineStore } from "pinia";
import { ref } from "vue";
import { errorMessage } from "../services/http";
import {
  changeMessages,
  createFolder,
  createLabel,
  deleteFolder,
  deleteLabel,
  fetchFolders,
  fetchLabels,
  fetchMessage,
  fetchMessages,
  sendForward,
  sendNewMessage,
  sendReply,
  splitAddresses,
  updateFolder,
  updateLabel,
  type DeskFolder,
} from "../services/message.service";
import { searchIsActive, searchMessages, type MailSearchInput } from "../services/search.service";
import { useUiStore } from "./ui";
import type { LabelTone, MailFilters, MailLabel, MailMessage, ThreadNote } from "../types/mail";

export interface MailSearchFields {
  from: string;
  to: string;
  subject: string;
  after: string;
  before: string;
  folder: string;
  labelId: string;
  allMailboxes: boolean;
}

export function emptySearch(): MailSearchFields {
  return {
    from: "",
    to: "",
    subject: "",
    after: "",
    before: "",
    folder: "",
    labelId: "",
    allMailboxes: false,
  };
}

export interface DeskTarget {
  id: string;
  address: string;
  displayName: string;
}

export const useMailStore = defineStore("mail", () => {
  const mailboxId = ref<string | null>(null);
  const address = ref("");
  const displayName = ref("");
  const view = ref("inbox");
  const folderId = ref<string | null>(null);
  const messages = ref<MailMessage[]>([]);
  const folders = ref<DeskFolder[]>([]);
  const labels = ref<MailLabel[]>([]);
  const detail = ref<MailMessage | null>(null);
  const thread = ref<ThreadNote[]>([]);
  const selectedIds = ref<string[]>([]);
  const activeLabelId = ref<string | null>(null);
  const nextCursor = ref<string | null>(null);
  const notice = ref("");
  const filters = ref<MailFilters>({
    unread: false,
    attachments: false,
    important: false,
  });
  const search = ref<MailSearchFields>(emptySearch());
  const searchActive = ref(false);
  const results = ref<MailMessage[]>([]);
  const searchNote = ref("");
  let listTicket = 0;
  let focusTicket = 0;
  let searchTicket = 0;

  function byId(id: string): MailMessage | undefined {
    return (
      messages.value.find((message) => message.id === id) ??
      results.value.find((message) => message.id === id)
    );
  }

  function clearDesk() {
    listTicket += 1;
    focusTicket += 1;
    mailboxId.value = null;
    address.value = "";
    displayName.value = "";
    messages.value = [];
    folders.value = [];
    labels.value = [];
    detail.value = null;
    thread.value = [];
    selectedIds.value = [];
    activeLabelId.value = null;
    nextCursor.value = null;
    notice.value = "";
  }

  async function load(target: DeskTarget, nextView: string, nextFolderId: string | null) {
    const mine = ++listTicket;
    mailboxId.value = target.id;
    address.value = target.address;
    displayName.value = target.displayName;
    view.value = nextView;
    folderId.value = nextFolderId;
    notice.value = "";
    selectedIds.value = [];
    try {
      const [folderRows, labelRows, page] = await Promise.all([
        fetchFolders(target.id),
        fetchLabels(target.id),
        fetchMessages(target.id, nextView, nextFolderId),
      ]);
      if (mine !== listTicket) {
        return;
      }
      folders.value = folderRows;
      labels.value = labelRows;
      messages.value = page.messages;
      nextCursor.value = page.nextCursor;
    } catch (error) {
      if (mine !== listTicket) {
        return;
      }
      messages.value = [];
      notice.value = errorMessage(error);
    }
  }

  async function reload() {
    if (!mailboxId.value) {
      return;
    }
    const mine = listTicket;
    const openId = detail.value?.id ?? null;
    try {
      const [folderRows, labelRows, page] = await Promise.all([
        fetchFolders(mailboxId.value),
        fetchLabels(mailboxId.value),
        fetchMessages(mailboxId.value, view.value, folderId.value),
      ]);
      if (mine !== listTicket) {
        return;
      }
      folders.value = folderRows;
      labels.value = labelRows;
      messages.value = page.messages;
      nextCursor.value = page.nextCursor;
      if (openId) {
        const opened = await fetchMessage(openId);
        if (mine !== listTicket) {
          return;
        }
        detail.value = opened.message;
        thread.value = opened.thread;
      }
    } catch (error) {
      if (mine === listTicket) {
        if (openId && detail.value?.id === openId) {
          detail.value = null;
          thread.value = [];
        }
        notice.value = errorMessage(error);
      }
    }
  }

  async function loadOlder() {
    if (!mailboxId.value || !nextCursor.value) {
      return;
    }
    const page = await fetchMessages(mailboxId.value, view.value, folderId.value, nextCursor.value);
    const seen = new Set(messages.value.map((message) => message.id));
    messages.value = [
      ...messages.value,
      ...page.messages.filter((message) => !seen.has(message.id)),
    ];
    nextCursor.value = page.nextCursor;
  }

  async function focus(id: string) {
    const mine = ++focusTicket;
    const box = mailboxId.value;
    notice.value = "";
    try {
      if (box) {
        await changeMessages(box, { ids: [id], action: "read", read: true });
      }
      const opened = await fetchMessage(id);
      if (mine !== focusTicket || box !== mailboxId.value) {
        return;
      }
      detail.value = opened.message;
      thread.value = opened.thread;
      const row = byId(id);
      if (row) {
        row.isRead = true;
      }
      if (box) {
        folders.value = await fetchFolders(box);
      }
    } catch (error) {
      if (mine === focusTicket) {
        notice.value = errorMessage(error);
      }
    }
  }

  function clearDetail() {
    detail.value = null;
    thread.value = [];
  }

  async function run(work: () => Promise<void>) {
    notice.value = "";
    try {
      await work();
      await reload();
      if (searchActive.value) {
        await applySearch(useUiStore().searchQuery);
      }
    } catch (error) {
      notice.value = errorMessage(error);
    }
  }

  function toggleSelected(id: string) {
    selectedIds.value = selectedIds.value.includes(id)
      ? selectedIds.value.filter((item) => item !== id)
      : [...selectedIds.value, id];
  }

  function selectAll(ids: string[]) {
    selectedIds.value = [...ids];
  }

  function clearSelected() {
    selectedIds.value = [];
  }

  async function setRead(ids: string[], read: boolean) {
    if (!mailboxId.value || ids.length === 0) {
      return;
    }
    await run(() => changeMessages(mailboxId.value!, { ids, action: "read", read }));
    if (detail.value && ids.includes(detail.value.id)) {
      detail.value = { ...detail.value, isRead: read };
    }
  }

  async function setStar(ids: string[], starred: boolean) {
    if (!mailboxId.value || ids.length === 0) {
      return;
    }
    await run(() => changeMessages(mailboxId.value!, { ids, action: "star", starred }));
  }

  async function toggleStar(id: string) {
    const message = byId(id) ?? (detail.value?.id === id ? detail.value : undefined);
    await setStar([id], !(message?.isStarred ?? false));
  }

  async function moveTo(ids: string[], target: string) {
    if (!mailboxId.value || ids.length === 0) {
      return;
    }
    const system = ["inbox", "sent", "drafts", "archive", "spam", "trash"];
    await run(() =>
      changeMessages(mailboxId.value!, {
        ids,
        action: "move",
        systemKey: system.includes(target) ? target : undefined,
        folderId: system.includes(target) ? undefined : target,
      }),
    );
    clearSelected();
  }

  async function deleteMessages(ids: string[]) {
    if (!mailboxId.value || ids.length === 0) {
      return;
    }
    await run(() => changeMessages(mailboxId.value!, { ids, action: "trash" }));
    clearSelected();
  }

  async function addLabel(ids: string[], labelId: string) {
    if (!mailboxId.value || ids.length === 0) {
      return;
    }
    await run(() => changeMessages(mailboxId.value!, { ids, action: "label", labelId }));
  }

  async function createNamedFolder(name: string, color: LabelTone | null) {
    if (!mailboxId.value) {
      return;
    }
    await run(() => createFolder(mailboxId.value!, name, color));
  }

  async function saveFolder(id: string, name: string, color: LabelTone | null) {
    await run(() => updateFolder(id, name, color));
  }

  async function removeFolder(id: string) {
    await run(() => deleteFolder(id));
  }

  async function createNamedLabel(name: string, color: LabelTone) {
    if (!mailboxId.value) {
      return;
    }
    await run(() => createLabel(mailboxId.value!, name, color));
  }

  async function saveLabel(id: string, name: string, color: LabelTone) {
    await run(() => updateLabel(id, name, color));
  }

  async function removeNamedLabel(id: string) {
    await run(async () => {
      await deleteLabel(id);
      if (activeLabelId.value === id) {
        activeLabelId.value = null;
      }
    });
  }

  async function deliver(input: {
    kind: "new" | "reply" | "reply-all" | "forward";
    mailboxId: string;
    messageId: string | null;
    draftId: string | null;
    to: string;
    cc: string;
    bcc: string;
    subject: string;
    text: string;
    html: string;
    groupIds?: string[];
  }) {
    const to = splitAddresses(input.to);
    const cc = splitAddresses(input.cc);
    const bcc = splitAddresses(input.bcc);
    const body = {
      to,
      cc,
      bcc,
      subject: input.subject,
      text: input.text,
      html: input.html,
      draftId: input.draftId,
      groupIds: input.groupIds ?? [],
    };
    if (input.kind === "new") {
      await sendNewMessage(input.mailboxId, body);
    } else if (!input.messageId) {
      throw new Error("Choose a message first.");
    } else if (input.kind === "forward") {
      await sendForward(input.messageId, body);
    } else {
      await sendReply(input.messageId, { ...body, all: input.kind === "reply-all" });
    }
    notice.value = "Accepted by the provider. Delivery is confirmed later.";
    await reload();
  }

  function toggleLabelFilter(labelId: string) {
    activeLabelId.value = activeLabelId.value === labelId ? null : labelId;
  }

  function clearLabelFilter() {
    activeLabelId.value = null;
  }

  function clearFilters() {
    filters.value = { unread: false, attachments: false, important: false };
    search.value = emptySearch();
  }

  function searchInput(query: string): MailSearchInput {
    return {
      query,
      from: search.value.from,
      to: search.value.to,
      subject: search.value.subject,
      after: search.value.after,
      before: search.value.before,
      folder: search.value.folder,
      labelId: search.value.labelId,
      mailboxId: search.value.allMailboxes ? "" : (mailboxId.value ?? ""),
      unread: filters.value.unread,
      attachments: filters.value.attachments,
      important: filters.value.important,
    };
  }

  async function applySearch(query: string) {
    const input = searchInput(query);
    if (!searchIsActive(input)) {
      searchTicket += 1;
      searchActive.value = false;
      results.value = [];
      searchNote.value = "";
      return;
    }
    const ticket = ++searchTicket;
    try {
      const page = await searchMessages(input);
      if (ticket !== searchTicket) {
        return;
      }
      results.value = page.messages;
      searchActive.value = true;
      searchNote.value = page.nextCursor ? "Showing the latest 50 matches." : "";
    } catch (error) {
      if (ticket !== searchTicket) {
        return;
      }
      results.value = [];
      searchActive.value = true;
      searchNote.value = errorMessage(error);
    }
  }

  async function ensureFolders(target: DeskTarget) {
    mailboxId.value = target.id;
    address.value = target.address;
    displayName.value = target.displayName;
    try {
      const [folderRows, labelRows] = await Promise.all([
        fetchFolders(target.id),
        fetchLabels(target.id),
      ]);
      folders.value = folderRows;
      labels.value = labelRows;
    } catch (error) {
      notice.value = errorMessage(error);
    }
  }

  function noteArrival(mailbox: string, inboxUnread: number) {
    if (mailboxId.value !== mailbox) {
      return;
    }
    const inbox = folders.value.find((folder) => folder.systemKey === "inbox");
    if (inbox) {
      inbox.unread = inboxUnread;
    }
    if (view.value === "inbox" && !folderId.value && !searchActive.value) {
      void reload();
    }
  }

  function unreadFor(key: string): number {
    return folders.value.find((folder) => folder.systemKey === key)?.unread ?? 0;
  }

  return {
    mailboxId,
    address,
    displayName,
    view,
    folderId,
    messages,
    folders,
    labels,
    detail,
    thread,
    selectedIds,
    activeLabelId,
    nextCursor,
    notice,
    filters,
    search,
    searchActive,
    results,
    searchNote,
    byId,
    clearDesk,
    load,
    reload,
    loadOlder,
    focus,
    clearDetail,
    toggleSelected,
    selectAll,
    clearSelected,
    setRead,
    setStar,
    toggleStar,
    moveTo,
    deleteMessages,
    addLabel,
    createNamedFolder,
    saveFolder,
    removeFolder,
    createNamedLabel,
    saveLabel,
    removeNamedLabel,
    deliver,
    toggleLabelFilter,
    clearLabelFilter,
    clearFilters,
    applySearch,
    ensureFolders,
    noteArrival,
    unreadFor,
  };
});
