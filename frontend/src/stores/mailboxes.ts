import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  chooseMailbox,
  createMailbox,
  fetchMailboxes,
  grantMailbox,
  makeDefaultMailbox,
  updateMailbox,
  type DeskMailbox,
} from "../services/mailbox.service";
import { useMailStore } from "./messages";

const STORAGE_KEY = "kamfany-mail.mailbox";

export const useMailboxStore = defineStore("mailboxes", () => {
  const items = ref<DeskMailbox[]>([]);
  const activeId = ref<string | null>(null);
  const loaded = ref(false);
  const notice = ref("");

  const active = computed(() => items.value.find((mailbox) => mailbox.id === activeId.value) ?? null);
  const readable = computed(() => items.value.filter((mailbox) => mailbox.canRead));

  function apply(next: DeskMailbox | null) {
    activeId.value = next?.id ?? null;
    if (next) {
      localStorage.setItem(STORAGE_KEY, next.id);
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
    if (!next?.canRead) {
      useMailStore().clearDesk();
    }
    notice.value = mailboxNotice(next);
  }

  async function ensure() {
    if (loaded.value) {
      return;
    }
    const stored = localStorage.getItem(STORAGE_KEY);
    items.value = await fetchMailboxes();
    loaded.value = true;
    apply(chooseMailbox(items.value, stored));
  }

  async function reload() {
    const stored = activeId.value ?? localStorage.getItem(STORAGE_KEY);
    items.value = await fetchMailboxes();
    loaded.value = true;
    apply(chooseMailbox(items.value, stored));
  }

  function select(id: string) {
    const next = readable.value.find((mailbox) => mailbox.id === id);
    if (next) {
      apply(next);
    }
  }

  async function register(address: string, displayName: string) {
    await createMailbox(address, displayName);
    await reload();
  }

  async function setEnabled(id: string, isEnabled: boolean) {
    await updateMailbox(id, { isEnabled });
    await reload();
  }

  async function setDefault(id: string) {
    await makeDefaultMailbox(id);
    await reload();
  }

  async function grant(
    id: string,
    input: { email: string; canRead: boolean; canSend: boolean; canManage: boolean },
  ) {
    await grantMailbox(id, input);
  }

  function clear() {
    items.value = [];
    loaded.value = false;
    apply(null);
  }

  return {
    items,
    active,
    readable,
    notice,
    loaded,
    ensure,
    reload,
    select,
    register,
    setEnabled,
    setDefault,
    grant,
    clear,
  };
});

function mailboxNotice(mailbox: DeskMailbox | null): string {
  if (!mailbox || mailbox.isReceivingVerified) {
    return "";
  }
  return "Receiving is not verified for this mailbox.";
}
