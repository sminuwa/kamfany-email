import type { ComposeKind } from "../types/mail";
import type { ApiData } from "../types/api";
import { http } from "./http";

export interface DraftFile {
  id: string;
  filename: string;
  contentType: string;
  sizeBytes: number;
  scanStatus?: string;
}

export interface ComposerDraft {
  id: string;
  mailboxId: string;
  kind: ComposeKind;
  inReplyTo: string | null;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  html: string;
  text: string;
  attachments: DraftFile[];
}

export interface MailSignature {
  id: string;
  mailboxId: string | null;
  name: string;
  html: string;
  isDefault: boolean;
}

export interface RecipientSuggestion {
  id?: string | null;
  address: string;
  name: string | null;
  kind?: string;
}

interface DraftJson {
  id: string;
  mailbox_id: string;
  compose_kind: ComposeKind;
  in_reply_to_message_id: string | null;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  body_html: string;
  body_text: string;
  attachments: {
    id: string;
    filename: string;
    content_type: string;
    size_bytes: number;
    scan_status?: string;
  }[];
}

interface SignatureJson {
  id: string;
  mailbox_id: string | null;
  name: string;
  body_html: string;
  is_default: boolean;
}

function draftFrom(raw: DraftJson): ComposerDraft {
  return {
    id: raw.id,
    mailboxId: raw.mailbox_id,
    kind: raw.compose_kind,
    inReplyTo: raw.in_reply_to_message_id,
    to: raw.to,
    cc: raw.cc,
    bcc: raw.bcc,
    subject: raw.subject,
    html: raw.body_html,
    text: raw.body_text,
    attachments: raw.attachments.map((file) => ({
      id: file.id,
      filename: file.filename,
      contentType: file.content_type,
      sizeBytes: file.size_bytes,
      scanStatus: file.scan_status ?? "pending",
    })),
  };
}

function signatureFrom(raw: SignatureJson): MailSignature {
  return {
    id: raw.id,
    mailboxId: raw.mailbox_id,
    name: raw.name,
    html: raw.body_html,
    isDefault: raw.is_default,
  };
}

export interface DraftFields {
  kind: ComposeKind;
  inReplyTo: string | null;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  html: string;
}

function draftBody(fields: DraftFields) {
  return {
    kind: fields.kind,
    in_reply_to: fields.inReplyTo,
    to: fields.to,
    cc: fields.cc,
    bcc: fields.bcc,
    subject: fields.subject,
    html: fields.html,
  };
}

export async function createDraft(mailboxId: string, fields: DraftFields): Promise<ComposerDraft> {
  const response = await http.post<ApiData<DraftJson>>(
    `/api/v1/mailboxes/${mailboxId}/drafts`,
    draftBody(fields),
  );
  return draftFrom(response.data.data);
}

export async function saveDraft(id: string, fields: DraftFields): Promise<ComposerDraft> {
  const response = await http.patch<ApiData<DraftJson>>(`/api/v1/drafts/${id}`, draftBody(fields));
  return draftFrom(response.data.data);
}

export async function fetchDraft(id: string): Promise<ComposerDraft> {
  const response = await http.get<ApiData<DraftJson>>(`/api/v1/drafts/${id}`);
  return draftFrom(response.data.data);
}

export async function deleteDraft(id: string): Promise<void> {
  await http.delete(`/api/v1/drafts/${id}`);
}

export async function uploadDraftFile(draftId: string, file: File): Promise<DraftFile> {
  const body = new FormData();
  body.append("file", file);
  const response = await http.post<ApiData<DraftJson["attachments"][number]>>(
    `/api/v1/drafts/${draftId}/attachments`,
    body,
    { timeout: 60_000 },
  );
  const raw = response.data.data;
  return {
    id: raw.id,
    filename: raw.filename,
    contentType: raw.content_type,
    sizeBytes: raw.size_bytes,
    scanStatus: raw.scan_status ?? "pending",
  };
}

export async function deleteDraftFile(draftId: string, fileId: string): Promise<void> {
  await http.delete(`/api/v1/drafts/${draftId}/attachments/${fileId}`);
}

export async function fetchSignatures(mailboxId: string): Promise<MailSignature[]> {
  const response = await http.get<ApiData<SignatureJson[]>>(
    `/api/v1/mailboxes/${mailboxId}/signatures`,
  );
  return response.data.data.map(signatureFrom);
}

export async function createSignature(input: {
  mailboxId: string | null;
  name: string;
  html: string;
  isDefault: boolean;
}): Promise<MailSignature> {
  const response = await http.post<ApiData<SignatureJson>>("/api/v1/signatures", {
    mailbox_id: input.mailboxId,
    name: input.name,
    html: input.html,
    is_default: input.isDefault,
  });
  return signatureFrom(response.data.data);
}

export async function deleteSignature(id: string): Promise<void> {
  await http.delete(`/api/v1/signatures/${id}`);
}

export async function suggestRecipients(
  mailboxId: string,
  query: string,
): Promise<RecipientSuggestion[]> {
  const response = await http.get<ApiData<RecipientSuggestion[]>>(
    `/api/v1/mailboxes/${mailboxId}/recipients`,
    { params: { q: query } },
  );
  return response.data.data;
}
