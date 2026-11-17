import { describe, expect, it } from "vitest";
import { snoozeChoices, snoozeInstant } from "./snooze";

describe("snooze times", () => {
  const mondayMorning = new Date(2026, 10, 16, 10, 0, 0);

  it("offers later today, tomorrow morning, and next Monday", () => {
    const choices = snoozeChoices(mondayMorning);
    expect(choices.map((choice) => choice.id)).toEqual(["later", "tomorrow", "week"]);
    expect(choices[0]?.until).toBe(new Date(2026, 10, 16, 18, 0, 0).toISOString());
    expect(choices[1]?.until).toBe(new Date(2026, 10, 17, 8, 0, 0).toISOString());
    expect(choices[2]?.until).toBe(new Date(2026, 10, 23, 8, 0, 0).toISOString());
  });

  it("drops later today once the evening is too close, and skips a duplicate Monday", () => {
    const evening = snoozeChoices(new Date(2026, 10, 16, 17, 30, 0));
    expect(evening.map((choice) => choice.id)).toEqual(["tomorrow", "week"]);
    const sunday = snoozeChoices(new Date(2026, 10, 15, 10, 0, 0));
    expect(sunday.map((choice) => choice.id)).toEqual(["later", "tomorrow"]);
    expect(sunday[1]?.until).toBe(new Date(2026, 10, 16, 8, 0, 0).toISOString());
  });

  it("accepts a future time within a year", () => {
    const now = new Date(2026, 10, 16, 10, 0, 0);
    expect(snoozeInstant("2026-11-16T09:00", now)).toBeNull();
    expect(snoozeInstant("2026-11-18T09:30", now)).toBe(new Date(2026, 10, 18, 9, 30, 0).toISOString());
    expect(snoozeInstant("2028-01-01T09:00", now)).toBeNull();
  });
});
