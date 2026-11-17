export interface MenuReadAction {
  read: boolean;
  label: string;
}

export function readActions(messages: { isRead: boolean }[]): MenuReadAction[] {
  const actions: MenuReadAction[] = [];
  if (messages.some((message) => !message.isRead)) {
    actions.push({ read: true, label: "Mark as read" });
  }
  if (messages.some((message) => message.isRead)) {
    actions.push({ read: false, label: "Mark as unread" });
  }
  return actions;
}

export function starAction(messages: { isStarred: boolean }[]): { starred: boolean; label: string } {
  const allStarred = messages.length > 0 && messages.every((message) => message.isStarred);
  return allStarred
    ? { starred: false, label: "Unstar" }
    : { starred: true, label: "Star" };
}

export function filingAction(messages: { folder: string }[]): { target: string; label: string } {
  const parked =
    messages.length > 0 &&
    messages.every(
      (message) => message.folder === "archive" || message.folder === "spam" || message.folder === "trash",
    );
  return parked
    ? { target: "inbox", label: "Move to inbox" }
    : { target: "archive", label: "Archive" };
}
