export interface SystemFolder {
  key: string;
  label: string;
  path: string;
  description: string;
}

export const systemFolders: SystemFolder[] = [
  {
    key: "inbox",
    label: "Inbox",
    path: "/",
    description: "New mail arrives here.",
  },
  {
    key: "starred",
    label: "Starred",
    path: "/starred",
    description: "Messages you have starred.",
  },
  {
    key: "snoozed",
    label: "Snoozed",
    path: "/snoozed",
    description: "Messages waiting until a later time.",
  },
  {
    key: "sent",
    label: "Sent",
    path: "/sent",
    description: "Mail sent from this desk.",
  },
  {
    key: "drafts",
    label: "Drafts",
    path: "/drafts",
    description: "Messages you have not sent.",
  },
  {
    key: "archive",
    label: "Archive",
    path: "/archive",
    description: "Messages filed out of the inbox.",
  },
  {
    key: "spam",
    label: "Spam",
    path: "/spam",
    description: "Messages marked as unwanted.",
  },
  {
    key: "trash",
    label: "Trash",
    path: "/trash",
    description: "Messages on their way out.",
  },
];

export function folderByKey(key: string | undefined): SystemFolder {
  return systemFolders.find((folder) => folder.key === key) ?? systemFolders[0]!;
}
