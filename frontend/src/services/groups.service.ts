import type { ApiData } from "../types/api";
import { http } from "./http";

interface GroupMailboxJson {
  id: string;
  address: string;
  display_name: string;
}

interface GroupJson {
  id: string;
  name: string;
  address: string;
  mailboxes: GroupMailboxJson[];
}

export interface GroupMailbox {
  id: string;
  address: string;
  displayName: string;
}

export interface MailGroup {
  id: string;
  name: string;
  address: string;
  mailboxes: GroupMailbox[];
}

function groupFrom(raw: GroupJson): MailGroup {
  return {
    id: raw.id,
    name: raw.name,
    address: raw.address,
    mailboxes: raw.mailboxes.map((mailbox) => ({
      id: mailbox.id,
      address: mailbox.address,
      displayName: mailbox.display_name,
    })),
  };
}

export async function fetchGroups(): Promise<MailGroup[]> {
  const response = await http.get<ApiData<GroupJson[]>>("/api/v1/groups");
  return response.data.data.map(groupFrom);
}

export async function createGroup(input: {
  name: string;
  address: string;
  mailboxIds: string[];
}): Promise<MailGroup> {
  const response = await http.post<ApiData<GroupJson>>("/api/v1/groups", {
    name: input.name,
    address: input.address,
    mailbox_ids: input.mailboxIds,
  });
  return groupFrom(response.data.data);
}

export async function updateGroup(
  id: string,
  input: { name?: string; mailboxIds?: string[] },
): Promise<MailGroup> {
  const response = await http.patch<ApiData<GroupJson>>(`/api/v1/groups/${id}`, {
    name: input.name,
    mailbox_ids: input.mailboxIds,
  });
  return groupFrom(response.data.data);
}

export async function deleteGroup(id: string): Promise<void> {
  await http.delete(`/api/v1/groups/${id}`);
}
