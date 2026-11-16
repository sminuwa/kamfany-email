import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  type AccountProfile,
  changePassword,
  createPerson,
  fetchMe,
  fetchPeople,
  fetchSessions,
  revokeSession,
  signIn,
  signOut,
  updateName,
  type DeskSession,
  type PersonAccount,
} from "../services/auth.service";
import axios from "axios";
import { useMailboxStore } from "./mailboxes";

export const useAuthStore = defineStore("auth", () => {
  const status = ref<"signed_out" | "signed_in">("signed_out");
  const user = ref<AccountProfile | null>(null);
  const checked = ref(false);
  const unreachable = ref(false);
  let inflight: Promise<void> | null = null;

  const canManageMailboxes = computed(() => user.value?.permissions.includes("mailbox.manage") ?? false);
  const canManageUsers = computed(() => user.value?.permissions.includes("users.manage") ?? false);
  const canReadAudit = computed(() => user.value?.permissions.includes("audit.read") ?? false);

  async function ensure() {
    if (checked.value) {
      return;
    }
    inflight ??= restore().finally(() => {
      inflight = null;
    });
    await inflight;
  }

  async function restore() {
    try {
      user.value = await fetchMe();
      status.value = "signed_in";
      unreachable.value = false;
    } catch (error) {
      user.value = null;
      status.value = "signed_out";
      unreachable.value = !(axios.isAxiosError(error) && error.response?.status === 401);
    } finally {
      checked.value = true;
    }
  }

  async function login(email: string, password: string) {
    user.value = await signIn(email, password);
    status.value = "signed_in";
    checked.value = true;
    unreachable.value = false;
  }

  async function logout() {
    try {
      await signOut();
    } catch {
      // The cookie is cleared locally even if the service is already gone.
    }
    user.value = null;
    status.value = "signed_out";
    checked.value = true;
    useMailboxStore().clear();
  }

  async function saveName(displayName: string) {
    user.value = await updateName(displayName);
  }

  async function savePassword(currentPassword: string, newPassword: string) {
    return changePassword(currentPassword, newPassword);
  }

  async function sessions(): Promise<DeskSession[]> {
    return fetchSessions();
  }

  async function endSession(id: string) {
    await revokeSession(id);
  }

  async function people(): Promise<PersonAccount[]> {
    return fetchPeople();
  }

  async function addPerson(input: {
    email: string;
    displayName: string;
    password: string;
    role: "member" | "administrator";
  }) {
    await createPerson(input);
  }

  return {
    status,
    user,
    checked,
    unreachable,
    canManageMailboxes,
    canManageUsers,
    canReadAudit,
    ensure,
    login,
    logout,
    saveName,
    savePassword,
    sessions,
    endSession,
    people,
    addPerson,
  };
});
