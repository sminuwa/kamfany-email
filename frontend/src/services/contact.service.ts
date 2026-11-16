import type { ApiData } from "../types/api";
import { http } from "./http";

export interface Contact {
  id: string;
  fullName: string;
  email: string;
  phone: string;
  company: string;
  notes: string;
  hasAvatar: boolean;
  updatedAt: string;
}

export interface ContactDraft {
  fullName: string;
  email: string;
  phone: string;
  company: string;
  notes: string;
}

export interface ImportResult {
  created: number;
  updated: number;
  skipped: number;
}

interface ContactJson {
  id: string;
  full_name: string;
  email: string | null;
  phone: string | null;
  company: string | null;
  notes: string | null;
  has_avatar: boolean;
  updated_at: string;
}

export function emptyContact(): ContactDraft {
  return { fullName: "", email: "", phone: "", company: "", notes: "" };
}

function contactFrom(raw: ContactJson): Contact {
  return {
    id: raw.id,
    fullName: raw.full_name,
    email: raw.email ?? "",
    phone: raw.phone ?? "",
    company: raw.company ?? "",
    notes: raw.notes ?? "",
    hasAvatar: raw.has_avatar,
    updatedAt: raw.updated_at,
  };
}

function bodyOf(draft: ContactDraft) {
  return {
    full_name: draft.fullName,
    email: draft.email,
    phone: draft.phone,
    company: draft.company,
    notes: draft.notes,
  };
}

export async function fetchContacts(query: string): Promise<Contact[]> {
  const response = await http.get<ApiData<ContactJson[]>>("/api/v1/contacts", {
    params: { q: query },
  });
  return response.data.data.map(contactFrom);
}

export async function createContact(draft: ContactDraft): Promise<Contact> {
  const response = await http.post<ApiData<ContactJson>>("/api/v1/contacts", bodyOf(draft));
  return contactFrom(response.data.data);
}

export async function saveContact(id: string, draft: ContactDraft): Promise<Contact> {
  const response = await http.patch<ApiData<ContactJson>>(`/api/v1/contacts/${id}`, bodyOf(draft));
  return contactFrom(response.data.data);
}

export async function deleteContact(id: string): Promise<void> {
  await http.delete(`/api/v1/contacts/${id}`);
}

export async function importContacts(file: File): Promise<ImportResult> {
  const body = new FormData();
  body.append("file", file);
  const response = await http.post<ApiData<{ created: number; updated: number; skipped: number }>>(
    "/api/v1/contacts/import",
    body,
    { timeout: 60_000 },
  );
  return response.data.data;
}

export async function uploadAvatar(id: string, file: File): Promise<Contact> {
  const body = new FormData();
  body.append("file", file);
  const response = await http.post<ApiData<ContactJson>>(`/api/v1/contacts/${id}/avatar`, body, {
    timeout: 60_000,
  });
  return contactFrom(response.data.data);
}

export async function deleteAvatar(id: string): Promise<void> {
  await http.delete(`/api/v1/contacts/${id}/avatar`);
}

export function avatarUrl(contact: Contact): string {
  return `/api/v1/contacts/${contact.id}/avatar?v=${encodeURIComponent(contact.updatedAt)}`;
}
