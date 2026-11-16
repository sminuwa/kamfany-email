export type OrganizationalFolder =
  | "inbox"
  | "sent"
  | "drafts"
  | "archive"
  | "spam"
  | "trash"
  | "custom";

export interface MailPerson {
  name: string;
  email: string;
}

export interface MailAttachment {
  id: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  scanStatus?: string;
}

export interface ThreadNote {
  id: string;
  fromName: string;
  fromEmail: string;
  subject: string;
  preview: string;
  activityAt: string;
  isRead: boolean;
}

export interface MailMessage {
  id: string;
  folder: OrganizationalFolder;
  folderId?: string;
  threadId?: string | null;
  threadCount?: number;
  direction?: "inbound" | "outbound";
  status?: string;
  from: MailPerson;
  to: MailPerson[];
  cc: MailPerson[];
  bcc: MailPerson[];
  subject: string;
  preview: string;
  bodyText: string;
  bodyHtml: string | null;
  receivedAt: string;
  isRead: boolean;
  isStarred: boolean;
  isImportant: boolean;
  snoozedUntil: string | null;
  hasAttachments?: boolean;
  labelIds: string[];
  attachments: MailAttachment[];
  mailboxAddress?: string;
}

export type LabelTone = "brass" | "paper" | "mist";

export interface MailLabel {
  id: string;
  name: string;
  tone: LabelTone;
}

export interface MailFilters {
  unread: boolean;
  attachments: boolean;
  important: boolean;
}

export type ComposeKind = "new" | "reply" | "reply-all" | "forward";

export interface ComposeIntent {
  kind: ComposeKind;
  messageId: string | null;
  draftId: string | null;
}
