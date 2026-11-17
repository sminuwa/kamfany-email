import { describe, expect, it } from "vitest";
import { filingAction, readActions, starAction } from "./message-menu";

describe("message menu actions", () => {
  it("offers the read state the selection does not already share", () => {
    expect(readActions([{ isRead: false }])).toEqual([{ read: true, label: "Mark as read" }]);
    expect(readActions([{ isRead: true }])).toEqual([{ read: false, label: "Mark as unread" }]);
    expect(readActions([{ isRead: true }, { isRead: false }])).toEqual([
      { read: true, label: "Mark as read" },
      { read: false, label: "Mark as unread" },
    ]);
  });

  it("stars a mixed selection and unstars one that is already starred", () => {
    expect(starAction([{ isStarred: false }]).label).toBe("Star");
    expect(starAction([{ isStarred: true }, { isStarred: true }])).toEqual({
      starred: false,
      label: "Unstar",
    });
  });

  it("returns parked mail to the inbox and archives the rest", () => {
    expect(filingAction([{ folder: "inbox" }])).toEqual({ target: "archive", label: "Archive" });
    expect(filingAction([{ folder: "trash" }, { folder: "spam" }])).toEqual({
      target: "inbox",
      label: "Move to inbox",
    });
    expect(filingAction([{ folder: "archive" }, { folder: "inbox" }]).label).toBe("Archive");
  });
});
