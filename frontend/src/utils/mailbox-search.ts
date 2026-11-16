export type MailboxStatus = "all" | "enabled" | "disabled";
export type MailboxSort = "newest" | "oldest" | "address";

export function mailboxMatches(mailbox: { address: string; displayName: string }, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return true;
  }
  return (
    mailbox.address.toLowerCase().includes(needle) || mailbox.displayName.toLowerCase().includes(needle)
  );
}

export function selectMailboxes<T extends { address: string; displayName: string; isEnabled: boolean; createdAt: string }>(
  items: T[],
  query: string,
  status: MailboxStatus,
  sort: MailboxSort,
): T[] {
  const matched = items.filter((mailbox) => {
    if (!mailboxMatches(mailbox, query)) {
      return false;
    }
    if (status === "enabled") {
      return mailbox.isEnabled;
    }
    if (status === "disabled") {
      return !mailbox.isEnabled;
    }
    return true;
  });
  return matched.sort((left, right) => {
    if (sort === "address") {
      return left.address.localeCompare(right.address);
    }
    const byTime = left.createdAt.localeCompare(right.createdAt);
    return sort === "oldest" ? byTime : -byTime;
  });
}
