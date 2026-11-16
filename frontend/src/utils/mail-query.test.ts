import { describe, expect, it } from "vitest";
import type { MailMessage, MailPerson } from "../types/mail";
import { filterMessages, inFolder, unreadInFolder } from "./mail-query";

const self: MailPerson = { name: "Desk", email: "support@kamfany.com" };
const now = Date.parse("2026-11-15T12:00:00.000Z");

function sample(overrides: Partial<MailMessage> & Pick<MailMessage, "id">): MailMessage {
  return {
    folder: "inbox",
    from: { name: "Adaeze Okonkwo", email: "finance@kamfany.com" },
    to: [self],
    cc: [],
    bcc: [],
    subject: "Budget",
    preview: "The support line",
    bodyText: "Please review the support line.",
    bodyHtml: null,
    receivedAt: "2026-11-15T08:00:00.000Z",
    isRead: true,
    isStarred: false,
    isImportant: false,
    snoozedUntil: null,
    labelIds: [],
    attachments: [],
    ...overrides,
  };
}

const messages = [
  sample({ id: "open", isRead: false, subject: "Harbour invoice", bodyText: "Payment is due." }),
  sample({
    id: "later",
    snoozedUntil: "2026-11-20T08:00:00.000Z",
    subject: "Monday follow up",
  }),
  sample({
    id: "kept",
    isStarred: true,
    folder: "archive",
    subject: "Provider comparison",
    attachments: [{ id: "file", filename: "notes.pdf", mimeType: "application/pdf", sizeBytes: 10 }],
    isImportant: true,
    labelIds: ["internal"],
  }),
];

const openFilters = { unread: false, attachments: false, important: false };

describe("mail query", () => {
  it("hides a snoozed message from the inbox and shows it under snoozed", () => {
    expect(inFolder(messages[1]!, "inbox", now)).toBe(false);
    expect(inFolder(messages[1]!, "snoozed", now)).toBe(true);
  });

  it("keeps starred mail in the starred view after it leaves the inbox", () => {
    expect(inFolder(messages[2]!, "starred", now)).toBe(true);
    expect(inFolder(messages[2]!, "archive", now)).toBe(true);
  });

  it("matches sender, subject, and body", () => {
    const found = filterMessages(messages, {
      folder: "inbox",
      query: "adaeze payment",
      filters: openFilters,
      labelId: null,
      now,
    });
    expect(found.map((message) => message.id)).toEqual(["open"]);
  });

  it("applies unread, attachment, important, and label filters", () => {
    const found = filterMessages(messages, {
      folder: "archive",
      query: "",
      filters: { unread: false, attachments: true, important: true },
      labelId: "internal",
      now,
    });
    expect(found.map((message) => message.id)).toEqual(["kept"]);
  });

  it("counts unread messages in the folder view", () => {
    expect(unreadInFolder(messages, "inbox", now)).toBe(1);
  });
});
