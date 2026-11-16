import {
  SAMPLE_ADDRESS,
  SAMPLE_NAME,
  sampleLabels,
  sampleMessages,
} from "../mocks/mailbox";
import type { ApiData } from "../types/api";
import type { LabelTone, MailLabel, MailMessage, OrganizationalFolder, ThreadNote } from "../types/mail";
import { http } from "./http";

export interface MailboxSnapshot {
  address: string;
  displayName: string;
  messages: MailMessage[];
  labels: MailLabel[];
}

export interface DeskFolder {
  id: string;
  systemKey: string | null;
  name: string;
  color: LabelTone | null;
  unread: number;
}

export interface MessagePage {
  messages: MailMessage[];
  nextCursor: string | null;
}

export interface MessageDetail {
  message: MailMessage;
  thread: ThreadNote[];
}

interface PersonJson {
  name: string | null;
  address: string;
}

interface AttachmentJson {
  id: string;
  filename: string;
  content_type: string;
  size_bytes: number;
  scan_status?: string;
}

export interface MessageJson {
  id: string;
  thread_id: string | null;
  thread_count: number;
  direction: "inbound" | "outbound";
  status: string;
  folder_id: string | null;
  folder_key: string | null;
  folder_name: string | null;
  from_address: string;
  from_name: string | null;
  to: PersonJson[];
  cc: PersonJson[];
  bcc: PersonJson[];
  subject: string;
  preview: string | null;
  activity_at: string;
  is_read: boolean;
  is_starred: boolean;
  is_important: boolean;
  snoozed_until: string | null;
  has_attachments: boolean;
  label_ids: string[];
  mailbox_address?: string | null;
  body_text?: string | null;
  body_html?: string | null;
  attachments?: AttachmentJson[];
}

interface FolderJson {
  id: string;
  system_key: string | null;
  name: string;
  color: string | null;
  unread: number;
}

interface LabelJson {
  id: string;
  name: string;
  color: string | null;
}

interface ThreadJson {
  id: string;
  from_address: string;
  from_name: string | null;
  subject: string;
  preview: string | null;
  activity_at: string;
  is_read: boolean;
}

const folders: OrganizationalFolder[] = ["inbox", "sent", "drafts", "archive", "spam", "trash"];

function toneOf(color: string | null): LabelTone | null {
  if (color === "brass" || color === "paper" || color === "mist") {
    return color;
  }
  return null;
}

function personFrom(raw: PersonJson) {
  return {
    name: raw.name?.trim() || raw.address,
    email: raw.address,
  };
}

export function messageFrom(raw: MessageJson): MailMessage {
  const folder = folders.find((item) => item === raw.folder_key) ?? "custom";
  return {
    id: raw.id,
    folder,
    folderId: raw.folder_id ?? undefined,
    threadId: raw.thread_id,
    threadCount: raw.thread_count,
    direction: raw.direction,
    status: raw.status,
    from: personFrom({ name: raw.from_name, address: raw.from_address }),
    to: raw.to.map(personFrom),
    cc: raw.cc.map(personFrom),
    bcc: raw.bcc.map(personFrom),
    subject: raw.subject,
    preview: raw.preview ?? "",
    bodyText: raw.body_text ?? raw.preview ?? "",
    bodyHtml: raw.body_html ?? null,
    receivedAt: raw.activity_at,
    isRead: raw.is_read,
    isStarred: raw.is_starred,
    isImportant: raw.is_important,
    snoozedUntil: raw.snoozed_until,
    hasAttachments: raw.has_attachments,
    labelIds: raw.label_ids,
    mailboxAddress: raw.mailbox_address ?? undefined,
    attachments: (raw.attachments ?? []).map((file) => ({
      id: file.id,
      filename: file.filename,
      mimeType: file.content_type,
      sizeBytes: file.size_bytes,
      scanStatus: file.scan_status ?? "pending",
    })),
  };
}

function folderFrom(raw: FolderJson): DeskFolder {
  return {
    id: raw.id,
    systemKey: raw.system_key,
    name: raw.name,
    color: toneOf(raw.color),
    unread: raw.unread,
  };
}

function labelFrom(raw: LabelJson): MailLabel {
  return {
    id: raw.id,
    name: raw.name,
    tone: toneOf(raw.color) ?? "paper",
  };
}

export async function fetchFolders(mailboxId: string): Promise<DeskFolder[]> {
  const response = await http.get<ApiData<FolderJson[]>>(`/api/v1/mailboxes/${mailboxId}/folders`);
  return response.data.data.map(folderFrom);
}

export async function fetchLabels(mailboxId: string): Promise<MailLabel[]> {
  const response = await http.get<ApiData<LabelJson[]>>(`/api/v1/mailboxes/${mailboxId}/labels`);
  return response.data.data.map(labelFrom);
}

export async function fetchMessages(
  mailboxId: string,
  view: string,
  folderId: string | null,
  cursor?: string | null,
): Promise<MessagePage> {
  const response = await http.get<ApiData<{ messages: MessageJson[]; next_cursor: string | null }>>(
    `/api/v1/mailboxes/${mailboxId}/messages`,
    {
      params: {
        view: folderId ? undefined : view,
        folder_id: folderId ?? undefined,
        cursor: cursor ?? undefined,
      },
    },
  );
  return {
    messages: response.data.data.messages.map(messageFrom),
    nextCursor: response.data.data.next_cursor,
  };
}

export async function fetchMessage(id: string): Promise<MessageDetail> {
  const response = await http.get<ApiData<{ message: MessageJson; thread: ThreadJson[] }>>(
    `/api/v1/messages/${id}`,
  );
  return {
    message: messageFrom(response.data.data.message),
    thread: response.data.data.thread.map((item) => ({
      id: item.id,
      fromName: item.from_name?.trim() || item.from_address,
      fromEmail: item.from_address,
      subject: item.subject,
      preview: item.preview ?? "",
      activityAt: item.activity_at,
      isRead: item.is_read,
    })),
  };
}

export async function changeMessages(
  mailboxId: string,
  body: {
    ids: string[];
    action: "read" | "star" | "move" | "trash" | "label" | "unlabel";
    read?: boolean;
    starred?: boolean;
    folderId?: string;
    systemKey?: string;
    labelId?: string;
  },
): Promise<void> {
  await http.post(`/api/v1/mailboxes/${mailboxId}/messages`, {
    ids: body.ids,
    action: body.action,
    read: body.read,
    starred: body.starred,
    folder_id: body.folderId,
    system_key: body.systemKey,
    label_id: body.labelId,
  });
}

export async function createFolder(mailboxId: string, name: string, color: LabelTone | null) {
  await http.post(`/api/v1/mailboxes/${mailboxId}/folders`, { name, color });
}

export async function updateFolder(id: string, name: string, color: LabelTone | null) {
  await http.patch(`/api/v1/folders/${id}`, { name, color: color ?? "" });
}

export async function deleteFolder(id: string) {
  await http.delete(`/api/v1/folders/${id}`);
}

export async function createLabel(mailboxId: string, name: string, color: LabelTone) {
  await http.post(`/api/v1/mailboxes/${mailboxId}/labels`, { name, color });
}

export async function updateLabel(id: string, name: string, color: LabelTone) {
  await http.patch(`/api/v1/labels/${id}`, { name, color });
}

export async function deleteLabel(id: string) {
  await http.delete(`/api/v1/labels/${id}`);
}

interface SendJson {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  text: string;
  html: string;
  draft_id?: string | null;
}

function sendBody(body: {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  text: string;
  html: string;
  draftId: string | null;
}): SendJson {
  return {
    to: body.to,
    cc: body.cc,
    bcc: body.bcc,
    subject: body.subject,
    text: body.text,
    html: body.html,
    draft_id: body.draftId,
  };
}

export async function sendNewMessage(
  mailboxId: string,
  body: {
    to: string[];
    cc: string[];
    bcc: string[];
    subject: string;
    text: string;
    html: string;
    draftId: string | null;
  },
) {
  await http.post(`/api/v1/mailboxes/${mailboxId}/send`, sendBody(body), { timeout: 60_000 });
}

export async function sendReply(
  messageId: string,
  body: {
    all: boolean;
    to: string[];
    cc: string[];
    bcc: string[];
    subject: string;
    text: string;
    html: string;
    draftId: string | null;
  },
) {
  await http.post(
    `/api/v1/messages/${messageId}/reply`,
    { ...sendBody(body), all: body.all },
    { timeout: 60_000 },
  );
}

export async function sendForward(
  messageId: string,
  body: {
    to: string[];
    cc: string[];
    bcc: string[];
    subject: string;
    text: string;
    html: string;
    draftId: string | null;
  },
) {
  await http.post(`/api/v1/messages/${messageId}/forward`, sendBody(body), { timeout: 60_000 });
}

export function splitAddresses(value: string): string[] {
  return value
    .split(/[,;]+/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

/** Sample records for compose and query tests. The desk reads the API. */
export function loadSampleMailbox(): MailboxSnapshot {
  return loadMailboxView(SAMPLE_ADDRESS);
}

export function loadMailboxView(address: string | null): MailboxSnapshot {
  if (address?.toLowerCase() === SAMPLE_ADDRESS) {
    return {
      address: SAMPLE_ADDRESS,
      displayName: SAMPLE_NAME,
      messages: structuredClone(sampleMessages),
      labels: structuredClone(sampleLabels),
    };
  }
  return {
    address: address ?? "",
    displayName: "",
    messages: [],
    labels: [],
  };
}
