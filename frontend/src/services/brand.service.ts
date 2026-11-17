import type { ApiData } from "../types/api";
import { http } from "./http";

interface BrandJson {
  name: string;
  company: string;
  domain: string;
  public_url: string;
  logo_url: string;
}

export interface BrandProfile {
  name: string;
  company: string;
  domain: string;
  publicUrl: string;
  logoUrl: string;
}

export async function fetchBrand(): Promise<BrandProfile> {
  const response = await http.get<ApiData<BrandJson>>("/api/v1/brand");
  const raw = response.data.data;
  return {
    name: raw.name,
    company: raw.company,
    domain: raw.domain,
    publicUrl: raw.public_url,
    logoUrl: raw.logo_url,
  };
}
