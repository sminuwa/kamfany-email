import type { ApiData } from "../types/api";
import type { MailMessage } from "../types/mail";
import { messageFrom, type MessageJson } from "./message.service";
import { http } from "./http";

export interface MailSearchInput {
  query: string;
  from: string;
  to: string;
  subject: string;
  after: string;
  before: string;
  folder: string;
  labelId: string;
  mailboxId: string;
  unread: boolean;
  attachments: boolean;
  important: boolean;
}

export interface SearchPage {
  messages: MailMessage[];
  nextCursor: string | null;
}

export function searchParams(input: MailSearchInput): URLSearchParams {
  const params = new URLSearchParams();
  const set = (key: string, value: string) => {
    const trimmed = value.trim();
    if (trimmed) {
      params.set(key, trimmed);
    }
  };
  set("q", input.query);
  set("from", input.from);
  set("to", input.to);
  set("subject", input.subject);
  set("after", input.after);
  set("before", input.before);
  set("folder", input.folder);
  set("label_id", input.labelId);
  set("mailbox_id", input.mailboxId);
  if (input.unread) {
    params.set("unread", "true");
  }
  if (input.attachments) {
    params.set("has_attachment", "true");
  }
  if (input.important) {
    params.set("important", "true");
  }
  return params;
}

export function searchIsActive(input: MailSearchInput): boolean {
  return [...searchParams(input).keys()].length > 0;
}

export async function searchMessages(input: MailSearchInput): Promise<SearchPage> {
  const response = await http.get<ApiData<{ messages: MessageJson[]; next_cursor: string | null }>>(
    "/api/v1/search",
    { params: searchParams(input) },
  );
  return {
    messages: response.data.data.messages.map(messageFrom),
    nextCursor: response.data.data.next_cursor,
  };
}
