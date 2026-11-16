import type { ApiData, HealthReport } from "../types/api";
import { http } from "./http";

export async function fetchHealth(): Promise<HealthReport> {
  const response = await http.get<ApiData<HealthReport>>("/api/v1/health");
  return response.data.data;
}
