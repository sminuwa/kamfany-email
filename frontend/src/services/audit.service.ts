import type { ApiData } from "../types/api";
import { http } from "./http";

interface AuditJson {
  id: string;
  action: string;
  entity_type: string | null;
  entity_id: string | null;
  actor_email: string | null;
  created_at: string;
}

export interface AuditEntry {
  id: string;
  action: string;
  entityType: string | null;
  entityId: string | null;
  actorEmail: string | null;
  createdAt: string;
}

export async function fetchAudit(): Promise<AuditEntry[]> {
  const response = await http.get<ApiData<AuditJson[]>>("/api/v1/audit");
  return response.data.data.map((row) => ({
    id: row.id,
    action: row.action,
    entityType: row.entity_type,
    entityId: row.entity_id,
    actorEmail: row.actor_email,
    createdAt: row.created_at,
  }));
}
