import dayjs from "dayjs";
import type { MailPerson } from "../types/mail";

export function formatListDate(iso: string, now = new Date()): string {
  const date = dayjs(iso);
  const today = dayjs(now);
  if (date.isSame(today, "day")) {
    return date.format("HH:mm");
  }
  if (date.isSame(today.subtract(1, "day"), "day")) {
    return "Yesterday";
  }
  if (date.isSame(today, "year")) {
    return date.format("D MMM");
  }
  return date.format("D MMM YYYY");
}

export function formatMessageDate(iso: string): string {
  return dayjs(iso).format("ddd D MMM YYYY, HH:mm");
}

export function scanNote(status: string | undefined): string | null {
  if (status === "rejected") {
    return "Rejected";
  }
  if (status === "failed") {
    return "Scan failed";
  }
  if (status === "pending") {
    return "Not scanned";
  }
  return null;
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatPeople(people: MailPerson[]): string {
  return people
    .map((person) =>
      person.name ? `${person.name} <${person.email}>` : person.email,
    )
    .join(", ");
}
