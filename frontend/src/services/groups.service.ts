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
  mailboxes: GroupMailbox[];
}

function groupFrom(raw: GroupJson): MailGroup {
  return {
    id: raw.id,
    name: raw.name,
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

export async function createGroup(name: string): Promise<MailGroup> {
  const response = await http.post<ApiData<GroupJson>>("/api/v1/groups", { name });
  return groupFrom(response.data.data);
}

export async function addGroupMailbox(id: string, mailboxId: string): Promise<MailGroup> {
  const response = await http.post<ApiData<GroupJson>>(`/api/v1/groups/${id}/mailboxes`, {
    mailbox_id: mailboxId,
  });
  return groupFrom(response.data.data);
}

export async function removeGroupMailbox(id: string, mailboxId: string): Promise<MailGroup> {
  const response = await http.delete<ApiData<GroupJson>>(`/api/v1/groups/${id}/mailboxes/${mailboxId}`);
  return groupFrom(response.data.data);
}

export interface GroupImport {
  added: number;
  alreadyInGroup: number;
  unknownCount: number;
  unknown: string[];
  notAdded: number;
  group: MailGroup;
}

export async function importGroupFile(id: string, file: File): Promise<GroupImport> {
  const body = new FormData();
  body.append("file", file);
  const response = await http.post<
    ApiData<{
      added: number;
      already_in_group: number;
      unknown_count: number;
      unknown: string[];
      not_added: number;
      group: GroupJson;
    }>
  >(`/api/v1/groups/${id}/import`, body, { timeout: 60_000 });
  const raw = response.data.data;
  return {
    added: raw.added,
    alreadyInGroup: raw.already_in_group,
    unknownCount: raw.unknown_count,
    unknown: raw.unknown,
    notAdded: raw.not_added,
    group: groupFrom(raw.group),
  };
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
