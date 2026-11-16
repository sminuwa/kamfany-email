import type { ApiData } from "../types/api";
import { http } from "./http";

interface MailboxJson {
  id: string;
  address: string;
  display_name: string;
  is_enabled: boolean;
  is_receiving_verified: boolean;
  can_read: boolean;
  can_send: boolean;
  can_manage: boolean;
  is_default: boolean;
}

export interface DeskMailbox {
  id: string;
  address: string;
  displayName: string;
  isEnabled: boolean;
  isReceivingVerified: boolean;
  canRead: boolean;
  canSend: boolean;
  canManage: boolean;
  isDefault: boolean;
}

function mailboxFrom(raw: MailboxJson): DeskMailbox {
  return {
    id: raw.id,
    address: raw.address,
    displayName: raw.display_name,
    isEnabled: raw.is_enabled,
    isReceivingVerified: raw.is_receiving_verified,
    canRead: raw.can_read,
    canSend: raw.can_send,
    canManage: raw.can_manage,
    isDefault: raw.is_default,
  };
}

export async function fetchMailboxes(): Promise<DeskMailbox[]> {
  const response = await http.get<ApiData<MailboxJson[]>>("/api/v1/mailboxes");
  return response.data.data.map(mailboxFrom);
}

export async function createMailbox(address: string, displayName: string): Promise<DeskMailbox> {
  const response = await http.post<ApiData<MailboxJson>>("/api/v1/mailboxes", {
    address,
    display_name: displayName,
  });
  return mailboxFrom(response.data.data);
}

export async function updateMailbox(
  id: string,
  patch: { displayName?: string; isEnabled?: boolean },
): Promise<DeskMailbox> {
  const response = await http.patch<ApiData<MailboxJson>>(`/api/v1/mailboxes/${id}`, {
    display_name: patch.displayName,
    is_enabled: patch.isEnabled,
  });
  return mailboxFrom(response.data.data);
}

export async function grantMailbox(
  id: string,
  input: { email: string; canRead: boolean; canSend: boolean; canManage: boolean },
): Promise<void> {
  await http.post(`/api/v1/mailboxes/${id}/members`, {
    email: input.email,
    can_read: input.canRead,
    can_send: input.canSend,
    can_manage: input.canManage,
  });
}

export async function makeDefaultMailbox(id: string): Promise<void> {
  await http.post(`/api/v1/mailboxes/${id}/default`);
}

export function chooseMailbox(
  mailboxes: DeskMailbox[],
  storedId: string | null,
): DeskMailbox | null {
  const readable = mailboxes.filter((mailbox) => mailbox.canRead);
  return (
    readable.find((mailbox) => mailbox.id === storedId) ??
    readable.find((mailbox) => mailbox.isDefault) ??
    readable[0] ??
    null
  );
}

export function safeReturnPath(value: unknown): string {
  if (typeof value !== "string" || !value.startsWith("/") || value.startsWith("//")) {
    return "/";
  }
  if (value.startsWith("/account")) {
    return "/";
  }
  return value;
}
