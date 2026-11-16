import { describe, expect, it } from "vitest";
import { pageCount, pageSlice } from "./pager";

describe("page lists", () => {
  it("keeps a short list on one page", () => {
    expect(pageCount(0)).toBe(1);
    expect(pageCount(10)).toBe(1);
    expect(pageSlice(["a", "b"], 1)).toEqual(["a", "b"]);
  });

  it("splits a long list into pages of ten", () => {
    const items = Array.from({ length: 25 }, (_, index) => index + 1);
    expect(pageCount(items.length)).toBe(3);
    expect(pageSlice(items, 1)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(pageSlice(items, 3)).toEqual([21, 22, 23, 24, 25]);
    expect(pageSlice(items, 9)).toEqual([21, 22, 23, 24, 25]);
  });
});
