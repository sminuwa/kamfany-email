import type { ApiData } from "../types/api";
import { http } from "./http";

export interface DeskNotice {
  id: string;
  mailboxId: string | null;
  messageId: string | null;
  title: string;
  body: string;
  readAt: string | null;
  createdAt: string;
}

interface NoticeJson {
  id: string;
  mailbox_id: string | null;
  message_id: string | null;
  title: string;
  body: string;
  read_at: string | null;
  created_at: string;
}

function noticeFrom(raw: NoticeJson): DeskNotice {
  return {
    id: raw.id,
    mailboxId: raw.mailbox_id,
    messageId: raw.message_id,
    title: raw.title,
    body: raw.body,
    readAt: raw.read_at,
    createdAt: raw.created_at,
  };
}

export async function fetchNotices(): Promise<DeskNotice[]> {
  const response = await http.get<ApiData<NoticeJson[]>>("/api/v1/notifications");
  return response.data.data.map(noticeFrom);
}

export async function markNotice(id: string): Promise<void> {
  await http.post(`/api/v1/notifications/${id}/read`);
}

export async function markNotices(): Promise<void> {
  await http.post("/api/v1/notifications/read");
}
