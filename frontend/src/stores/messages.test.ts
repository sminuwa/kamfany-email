import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { changeMessages, fetchFolders, fetchLabels, fetchMessages } from "../services/message.service";
import { useMailStore } from "./messages";

vi.mock("../services/message.service", () => ({
  fetchFolders: vi.fn().mockResolvedValue([]),
  fetchLabels: vi.fn().mockResolvedValue([]),
  fetchMessages: vi.fn().mockResolvedValue({ messages: [], nextCursor: null }),
  fetchMessage: vi.fn(),
  changeMessages: vi.fn().mockResolvedValue(undefined),
  createFolder: vi.fn(),
  updateFolder: vi.fn(),
  deleteFolder: vi.fn(),
  createLabel: vi.fn(),
  updateLabel: vi.fn(),
  deleteLabel: vi.fn(),
  sendNewMessage: vi.fn(),
  sendReply: vi.fn(),
  sendForward: vi.fn(),
  splitAddresses: (value: string) =>
    value
      .split(/[,;]+/)
      .map((item) => item.trim())
      .filter((item) => item.length > 0),
}));

const target = {
  id: "box-1",
  address: "finance@kamfany.com",
  displayName: "Finance",
};

describe("mail store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(fetchMessages).mockClear();
    vi.mocked(changeMessages).mockClear();
  });

  it("loads the open mailbox from the API", async () => {
    const mail = useMailStore();
    await mail.load(target, "inbox", null);
    expect(mail.address).toBe("finance@kamfany.com");
    expect(mail.messages).toEqual([]);
    expect(fetchFolders).toHaveBeenCalledWith("box-1");
    expect(fetchLabels).toHaveBeenCalledWith("box-1");
    expect(fetchMessages).toHaveBeenCalledWith("box-1", "inbox", null);
  });

  it("asks the API to archive the selected messages", async () => {
    const mail = useMailStore();
    await mail.load(target, "inbox", null);
    await mail.moveTo(["message-1"], "archive");
    expect(changeMessages).toHaveBeenCalledWith("box-1", {
      ids: ["message-1"],
      action: "move",
      systemKey: "archive",
      folderId: undefined,
    });
  });
});
