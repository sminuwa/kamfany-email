import { describe, expect, it } from "vitest";
import { searchIsActive, searchParams } from "./search.service";

const blank = {
  query: "",
  from: "",
  to: "",
  subject: "",
  after: "",
  before: "",
  folder: "",
  labelId: "",
  mailboxId: "",
  unread: false,
  attachments: false,
  important: false,
};

describe("search params", () => {
  it("keeps an empty desk search off the server", () => {
    expect(searchIsActive(blank)).toBe(false);
  });

  it("sends the filled fields", () => {
    const params = searchParams({
      ...blank,
      query: "harbour",
      from: "ada@kamfany.com",
      mailboxId: "box",
      unread: true,
    });
    expect(params.get("q")).toBe("harbour");
    expect(params.get("from")).toBe("ada@kamfany.com");
    expect(params.get("mailbox_id")).toBe("box");
    expect(params.get("unread")).toBe("true");
    expect(params.has("to")).toBe(false);
  });
});
