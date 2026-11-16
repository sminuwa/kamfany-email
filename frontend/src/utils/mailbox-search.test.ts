import { describe, expect, it } from "vitest";
import { mailboxMatches, selectMailboxes } from "./mailbox-search";

const finance = { address: "finance@kamfany.com", displayName: "KAMFANY Finance" };

describe("mailbox search", () => {
  it("keeps every mailbox when the search is empty", () => {
    expect(mailboxMatches(finance, "  ")).toBe(true);
  });

  it("matches the address or the display name", () => {
    expect(mailboxMatches(finance, "FINANCE@")).toBe(true);
    expect(mailboxMatches(finance, "finance")).toBe(true);
    expect(mailboxMatches(finance, "support")).toBe(false);
  });

  it("filters by status and sorts newest first", () => {
    const rows = [
      { address: "a@kamfany.com", displayName: "A", isEnabled: true, createdAt: "2026-01-01T00:00:00Z" },
      { address: "b@kamfany.com", displayName: "B", isEnabled: false, createdAt: "2026-03-01T00:00:00Z" },
    ];
    expect(selectMailboxes(rows, "", "disabled", "newest").map((row) => row.address)).toEqual(["b@kamfany.com"]);
    expect(selectMailboxes(rows, "", "all", "newest").map((row) => row.address)).toEqual([
      "b@kamfany.com",
      "a@kamfany.com",
    ]);
  });
});
