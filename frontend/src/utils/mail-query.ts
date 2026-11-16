import type { MailFilters, MailMessage } from "../types/mail";

export const PAGE_SIZE = 8;

export interface MailQuery {
  folder: string;
  folderId?: string | null;
  query: string;
  filters: MailFilters;
  labelId: string | null;
  now?: number;
}

export function isSnoozed(message: MailMessage, now: number): boolean {
  if (!message.snoozedUntil) {
    return false;
  }
  return new Date(message.snoozedUntil).getTime() > now;
}

export function inFolder(
  message: MailMessage,
  folder: string,
  now: number,
): boolean {
  if (folder === "starred") {
    return message.isStarred && message.folder !== "trash";
  }
  if (folder === "snoozed") {
    return isSnoozed(message, now) && message.folder !== "trash";
  }
  if (message.folder !== folder) {
    return false;
  }
  if (folder === "inbox" && isSnoozed(message, now)) {
    return false;
  }
  return true;
}

export function matchesQuery(message: MailMessage, query: string): boolean {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) {
    return true;
  }
  const people = [message.from, ...message.to, ...message.cc, ...message.bcc];
  const haystack = [
    message.subject,
    message.preview,
    message.bodyText,
    ...people.flatMap((person) => [person.name, person.email]),
  ]
    .join(" ")
    .toLowerCase();
  return terms.every((term) => haystack.includes(term));
}

export function matchesFilters(
  message: MailMessage,
  filters: MailFilters,
  labelId: string | null,
): boolean {
  if (filters.unread && message.isRead) {
    return false;
  }
  const attached = message.hasAttachments || message.attachments.length > 0;
  if (filters.attachments && !attached) {
    return false;
  }
  if (filters.important && !message.isImportant) {
    return false;
  }
  if (labelId && !message.labelIds.includes(labelId)) {
    return false;
  }
  return true;
}

export function filterMessages(
  messages: MailMessage[],
  query: MailQuery,
): MailMessage[] {
  const now = query.now ?? Date.now();
  return messages
    .filter((message) =>
      query.folderId ? message.folderId === query.folderId : inFolder(message, query.folder, now),
    )
    .filter((message) => matchesQuery(message, query.query))
    .filter((message) => matchesFilters(message, query.filters, query.labelId))
    .sort(
      (left, right) =>
        new Date(right.receivedAt).getTime() - new Date(left.receivedAt).getTime(),
    );
}

export function unreadInFolder(
  messages: MailMessage[],
  folder: string,
  now = Date.now(),
): number {
  return messages.filter(
    (message) => inFolder(message, folder, now) && !message.isRead,
  ).length;
}
