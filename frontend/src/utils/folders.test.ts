import { describe, expect, it } from "vitest";
import { folderByKey, systemFolders } from "./folders";

describe("systemFolders", () => {
  it("starts at the inbox and keeps every path unique", () => {
    expect(systemFolders[0]?.path).toBe("/");
    expect(new Set(systemFolders.map((folder) => folder.path)).size).toBe(systemFolders.length);
    expect(new Set(systemFolders.map((folder) => folder.key)).size).toBe(systemFolders.length);
  });

  it("falls back to the inbox for an unknown key", () => {
    expect(folderByKey(undefined).key).toBe("inbox");
    expect(folderByKey("missing").key).toBe("inbox");
  });
});
