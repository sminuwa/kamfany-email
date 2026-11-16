import { describe, expect, it } from "vitest";
import { loadSampleMailbox } from "../services/message.service";
import { prepareCompose } from "./compose";

describe("prepare compose", () => {
  const mailbox = loadSampleMailbox();
  const inbound = mailbox.messages.find((message) => message.id === "m04")!;
  const sent = mailbox.messages.find((message) => message.id === "m11")!;

  it("prepares a reply to the sender", () => {
    const draft = prepareCompose("reply", inbound, mailbox.address);
    expect(draft.title).toBe("Reply");
    expect(draft.to).toContain("amaka@northwind.example");
    expect(draft.cc).toBe("");
    expect(draft.subject).toBe("Re: Northwind queue from Monday");
    expect(draft.body).toContain("Saturday");
    expect(draft.note).toContain("formatted reply");
  });

  it("prepares reply all with the other recipients", () => {
    const draft = prepareCompose("reply-all", inbound, mailbox.address);
    expect(draft.title).toBe("Reply all");
    expect(draft.to).toContain("amaka@northwind.example");
    expect(draft.cc).toContain("info@kamfany.com");
    expect(draft.cc).not.toContain(mailbox.address);
  });

  it("prepares a forward without a recipient", () => {
    const draft = prepareCompose("forward", inbound, mailbox.address);
    expect(draft.title).toBe("Forward");
    expect(draft.to).toBe("");
    expect(draft.subject.startsWith("Fwd:")).toBe(true);
    expect(draft.body).toContain(inbound.from.email);
  });

  it("replies to the recipients of a sent message and keeps bcc private", () => {
    const draft = prepareCompose("reply-all", sent, mailbox.address);
    expect(draft.to).toContain("info@kamfany.com");
    expect(draft.cc).not.toContain("finance@kamfany.com");
    expect(draft.body).not.toContain("Bcc");
  });

  it("starts a blank message when nothing is being answered", () => {
    const draft = prepareCompose("new", null, mailbox.address);
    expect(draft.title).toBe("New message");
    expect(draft.to).toBe("");
    expect(draft.subject).toBe("");
    expect(draft.note).toContain("formatted mail");
  });
});
