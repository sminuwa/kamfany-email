import { describe, expect, it } from "vitest";
import type { DeskMailbox } from "./mailbox.service";
import { chooseMailbox, safeReturnPath } from "./mailbox.service";

function box(overrides: Partial<DeskMailbox> & Pick<DeskMailbox, "id">): DeskMailbox {
  return {
    address: "support@kamfany.com",
    displayName: "Support",
    isEnabled: false,
    isReceivingVerified: false,
    canRead: true,
    canSend: false,
    canManage: false,
    isDefault: false,
    ...overrides,
  };
}

describe("mailbox choice", () => {
  const mailboxes = [
    box({ id: "finance", address: "finance@kamfany.com", canRead: false, canManage: true }),
    box({ id: "support", isDefault: true }),
    box({ id: "info", address: "info@kamfany.com" }),
  ];

  it("keeps a stored mailbox the account can read", () => {
    expect(chooseMailbox(mailboxes, "info")?.id).toBe("info");
  });

  it("falls back to the default when the stored mailbox is not readable", () => {
    expect(chooseMailbox(mailboxes, "finance")?.id).toBe("support");
    expect(chooseMailbox(mailboxes, null)?.id).toBe("support");
  });

  it("refuses an off-site return path", () => {
    expect(safeReturnPath("/drafts")).toBe("/drafts");
    expect(safeReturnPath("https://evil.example")).toBe("/");
    expect(safeReturnPath("//evil.example")).toBe("/");
    expect(safeReturnPath("/account")).toBe("/");
  });
});
