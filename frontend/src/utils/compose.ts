import type { ComposeKind, MailMessage, MailPerson } from "../types/mail";
import { formatMessageDate } from "./format-mail";

export interface PreparedCompose {
  title: string;
  to: string;
  cc: string;
  subject: string;
  body: string;
  note: string;
}

const NEW_NOTE = "This sends formatted mail from your mailbox.";
const REPLY_NOTE = "This sends a formatted reply on the same thread.";
const FORWARD_NOTE = "This sends a formatted forward as a new message.";

function withPrefix(subject: string, prefix: "Re:" | "Fwd:"): string {
  const pattern = prefix === "Re:" ? /^re:/i : /^fwd:/i;
  if (pattern.test(subject.trim())) {
    return subject;
  }
  return `${prefix} ${subject}`;
}

function quote(message: MailMessage): string {
  const when = formatMessageDate(message.receivedAt);
  return `${message.from.name} <${message.from.email}> wrote on ${when}:\n\n${message.bodyText}`;
}

function replyRecipients(
  message: MailMessage,
  selfEmail: string,
  all: boolean,
): { to: MailPerson[]; cc: MailPerson[] } {
  const self = selfEmail.toLowerCase();
  const fromSelf = message.from.email.toLowerCase() === self;
  const primary = fromSelf
    ? message.to.filter((person) => person.email.toLowerCase() !== self)
    : [message.from];
  if (!all) {
    return { to: primary, cc: [] };
  }
  const seen = new Set(primary.map((person) => person.email.toLowerCase()));
  seen.add(self);
  const cc: MailPerson[] = [];
  const others = fromSelf ? message.cc : [...message.to, ...message.cc];
  for (const person of others) {
    const key = person.email.toLowerCase();
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    cc.push(person);
  }
  return { to: primary, cc };
}

export function prepareCompose(
  kind: ComposeKind,
  message: MailMessage | null,
  selfEmail: string,
): PreparedCompose {
  if (kind === "new" || !message) {
    return {
      title: "New message",
      to: "",
      cc: "",
      subject: "",
      body: "",
      note: NEW_NOTE,
    };
  }
  if (kind === "forward") {
    return {
      title: "Forward",
      to: "",
      cc: "",
      subject: withPrefix(message.subject, "Fwd:"),
      body: `\n\n${quote(message)}`,
      note: FORWARD_NOTE,
    };
  }
  const recipients = replyRecipients(message, selfEmail, kind === "reply-all");
  return {
    title: kind === "reply-all" ? "Reply all" : "Reply",
      to: recipients.to.map((person) => person.email).join(", "),
      cc: recipients.cc.map((person) => person.email).join(", "),
    subject: withPrefix(message.subject, "Re:"),
    body: `\n\n${quote(message)}`,
    note: REPLY_NOTE,
  };
}
