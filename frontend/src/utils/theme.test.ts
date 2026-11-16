import { describe, expect, it } from "vitest";
import { applyTheme, resolveTheme } from "./theme";

describe("resolveTheme", () => {
  it("follows the system when asked", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("keeps an explicit choice", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

describe("applyTheme", () => {
  it("writes the resolved theme onto the document", () => {
    document.head.innerHTML = '<meta name="theme-color" content="#000000" />';
    applyTheme("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
    expect(document.querySelector('meta[name="theme-color"]')?.getAttribute("content")).toBe(
      "#082221",
    );
  });
});
