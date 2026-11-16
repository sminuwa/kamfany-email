import type { ApiData } from "../types/api";
import { http } from "./http";

interface ProviderJson {
  configured: boolean;
  webhook_configured: boolean;
  sending_domain: string;
  domain_status: string;
  receiving: string;
  verified_addresses: string[];
  scanner: string;
  queue_received: number;
  queue_processing: number;
  queue_failed: number;
  queue_oldest_seconds: number | null;
}

export interface ProviderStatus {
  configured: boolean;
  webhookConfigured: boolean;
  sendingDomain: string;
  domainStatus: string;
  receiving: string;
  verifiedAddresses: string[];
  scanner: string;
  queueReceived: number;
  queueProcessing: number;
  queueFailed: number;
  queueOldestSeconds: number | null;
}

function fromJson(raw: ProviderJson): ProviderStatus {
  return {
    configured: raw.configured,
    webhookConfigured: raw.webhook_configured,
    sendingDomain: raw.sending_domain,
    domainStatus: raw.domain_status,
    receiving: raw.receiving,
    verifiedAddresses: raw.verified_addresses,
    scanner: raw.scanner,
    queueReceived: raw.queue_received,
    queueProcessing: raw.queue_processing,
    queueFailed: raw.queue_failed,
    queueOldestSeconds: raw.queue_oldest_seconds,
  };
}

export async function fetchProviderStatus(): Promise<ProviderStatus> {
  const response = await http.get<ApiData<ProviderJson>>("/api/v1/provider");
  return fromJson(response.data.data);
}

export async function syncProvider(): Promise<ProviderStatus> {
  const response = await http.post<ApiData<ProviderJson>>("/api/v1/provider");
  return fromJson(response.data.data);
}
