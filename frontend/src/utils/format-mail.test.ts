import { describe, expect, it } from "vitest";
import { formatFileSize, formatListDate } from "./format-mail";

describe("format mail", () => {
  const now = new Date(2026, 10, 15, 18, 0, 0);

  it("uses the clock for today", () => {
    const morning = new Date(2026, 10, 15, 8, 5, 0);
    expect(formatListDate(morning.toISOString(), now)).toBe("08:05");
  });

  it("labels yesterday", () => {
    const yesterday = new Date(2026, 10, 14, 9, 0, 0);
    expect(formatListDate(yesterday.toISOString(), now)).toBe("Yesterday");
  });

  it("uses the day and month inside the year", () => {
    const older = new Date(2026, 9, 2, 9, 0, 0);
    expect(formatListDate(older.toISOString(), now)).toBe("2 Oct");
  });

  it("includes the year when the message is older", () => {
    const lastYear = new Date(2024, 0, 3, 9, 0, 0);
    expect(formatListDate(lastYear.toISOString(), now)).toBe("3 Jan 2024");
  });

  it("rounds file sizes", () => {
    expect(formatFileSize(180)).toBe("180 B");
    expect(formatFileSize(184_320)).toBe("180 KB");
    expect(formatFileSize(1_258_291)).toBe("1.2 MB");
  });
});
