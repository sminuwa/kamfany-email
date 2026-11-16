import type { ApiData } from "../types/api";
import { http } from "./http";

interface ProfileJson {
  id: string;
  email: string;
  display_name: string;
  roles: string[];
  permissions: string[];
}

interface SessionJson {
  id: string;
  user_agent: string | null;
  created_at: string;
  expires_at: string;
  current: boolean;
}

export interface AccountProfile {
  id: string;
  email: string;
  displayName: string;
  roles: string[];
  permissions: string[];
}

export interface DeskSession {
  id: string;
  userAgent: string | null;
  createdAt: string;
  expiresAt: string;
  current: boolean;
}

export interface PersonAccount {
  id: string;
  email: string;
  displayName: string;
  roles: string[];
}

function profileFrom(raw: ProfileJson): AccountProfile {
  return {
    id: raw.id,
    email: raw.email,
    displayName: raw.display_name,
    roles: raw.roles,
    permissions: raw.permissions,
  };
}

export async function fetchMe(): Promise<AccountProfile> {
  const response = await http.get<ApiData<ProfileJson>>("/api/v1/auth/me");
  return profileFrom(response.data.data);
}

export async function signIn(email: string, password: string): Promise<AccountProfile> {
  const response = await http.post<ApiData<ProfileJson>>("/api/v1/auth/login", {
    email,
    password,
  });
  return profileFrom(response.data.data);
}

export async function signOut(): Promise<void> {
  await http.post("/api/v1/auth/logout");
}

export async function updateName(displayName: string): Promise<AccountProfile> {
  const response = await http.patch<ApiData<ProfileJson>>("/api/v1/auth/me", {
    display_name: displayName,
  });
  return profileFrom(response.data.data);
}

export async function changePassword(currentPassword: string, newPassword: string): Promise<string> {
  const response = await http.post<ApiData<{ message: string }>>("/api/v1/auth/password", {
    current_password: currentPassword,
    new_password: newPassword,
  });
  return response.data.data.message;
}

export async function fetchSessions(): Promise<DeskSession[]> {
  const response = await http.get<ApiData<SessionJson[]>>("/api/v1/auth/sessions");
  return response.data.data.map((session) => ({
    id: session.id,
    userAgent: session.user_agent,
    createdAt: session.created_at,
    expiresAt: session.expires_at,
    current: session.current,
  }));
}

export async function revokeSession(id: string): Promise<void> {
  await http.delete(`/api/v1/auth/sessions/${id}`);
}

export async function fetchPeople(): Promise<PersonAccount[]> {
  const response = await http.get<ApiData<ProfileJson[]>>("/api/v1/users");
  return response.data.data.map((person) => ({
    id: person.id,
    email: person.email,
    displayName: person.display_name,
    roles: person.roles,
  }));
}

export async function createPerson(input: {
  email: string;
  displayName: string;
  password: string;
  role: "member" | "administrator";
}): Promise<void> {
  await http.post("/api/v1/users", {
    email: input.email,
    display_name: input.displayName,
    password: input.password,
    role: input.role,
  });
}
