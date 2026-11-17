import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import type { MailMessage } from "../../types/mail";
import MessageContextMenu from "./MessageContextMenu.vue";
import MessageRow from "./MessageRow.vue";

const message: MailMessage = {
  id: "m1",
  folder: "inbox",
  from: { name: "Ada", email: "ada@example.com" },
  to: [],
  cc: [],
  bcc: [],
  subject: "Quarterly note",
  preview: "See the attached note.",
  bodyText: "",
  bodyHtml: null,
  receivedAt: "2026-11-17T08:00:00Z",
  isRead: false,
  isStarred: false,
  isImportant: false,
  snoozedUntil: null,
  labelIds: [],
  attachments: [],
};

describe("MessageContextMenu", () => {
  it("lists the message actions and keeps reply off a draft", async () => {
    const wrapper = mount(MessageContextMenu, {
      attachTo: document.body,
      props: {
        x: 20,
        y: 20,
        subject: "Quarterly note",
        count: 1,
        draft: false,
        filingLabel: "Archive",
        filingTarget: "archive",
        starLabel: "Star",
        readActions: [{ read: true, label: "Mark as read" }],
        folders: [{ id: "folder-1", name: "Clients" }],
        labels: [{ id: "label-1", name: "Billing" }],
        canSnooze: true,
      },
    });

    const text = document.body.textContent ?? "";
    expect(text).toContain("Reply");
    expect(text).toContain("Reply all");
    expect(text).toContain("Forward");
    expect(text).toContain("Mark as read");
    expect(text).toContain("Archive");
    expect(text).toContain("Snooze");
    expect(text).toContain("Delete");

    const menu = document.body.querySelector("[role='menu']");
    const move = [...(menu?.querySelectorAll("button") ?? [])].find((button) =>
      button.textContent?.includes("Move"),
    );
    move?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).toContain("Clients");
    const label = [...(menu?.querySelectorAll("button") ?? [])].find((button) =>
      button.textContent?.includes("Label"),
    );
    label?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).toContain("Billing");
    const snooze = [...(menu?.querySelectorAll("button") ?? [])].find((button) =>
      button.textContent?.includes("Snooze"),
    );
    snooze?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).toContain("Tomorrow");

    await wrapper.setProps({ draft: true });
    expect(document.body.textContent).not.toContain("Reply");
    wrapper.unmount();
  });
});

describe("MessageRow", () => {
  it("opens the message menu on a right click", async () => {
    const wrapper = mount(MessageRow, {
      props: { message, labels: [], selected: false, open: false },
    });
    await wrapper.get("[role='option']").trigger("contextmenu");
    expect(wrapper.emitted("menu")).toHaveLength(1);
  });
});
