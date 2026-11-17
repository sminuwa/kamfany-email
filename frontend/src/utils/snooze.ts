import dayjs from "dayjs";

export interface SnoozeChoice {
  id: "later" | "tomorrow" | "week";
  label: string;
  until: string;
}

export function snoozeChoices(now = new Date()): SnoozeChoice[] {
  const choices: SnoozeChoice[] = [];
  const later = dayjs(now).hour(18).minute(0).second(0).millisecond(0);
  if (later.diff(dayjs(now), "minute") >= 60) {
    choices.push({
      id: "later",
      label: `Later today, ${later.format("HH:mm")}`,
      until: later.toISOString(),
    });
  }
  const tomorrow = dayjs(now).add(1, "day").hour(8).minute(0).second(0).millisecond(0);
  choices.push({
    id: "tomorrow",
    label: `Tomorrow, ${tomorrow.format("HH:mm")}`,
    until: tomorrow.toISOString(),
  });
  const weekday = dayjs(now).day();
  const daysUntilMonday = weekday === 0 ? 1 : weekday === 1 ? 7 : 8 - weekday;
  const week = dayjs(now).add(daysUntilMonday, "day").hour(8).minute(0).second(0).millisecond(0);
  if (!week.isSame(tomorrow)) {
    choices.push({
      id: "week",
      label: `Next week, ${week.format("ddd HH:mm")}`,
      until: week.toISOString(),
    });
  }
  return choices;
}

export function snoozeInstant(value: string, now = new Date()): string | null {
  const wake = dayjs(value);
  if (!wake.isValid()) {
    return null;
  }
  if (!wake.isAfter(dayjs(now))) {
    return null;
  }
  if (wake.isAfter(dayjs(now).add(366, "day"))) {
    return null;
  }
  return wake.toISOString();
}
