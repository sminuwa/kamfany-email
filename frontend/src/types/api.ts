export interface ApiData<T> {
  data: T;
}

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
  };
}

export interface HealthReport {
  status: string;
  service: string;
  version: string;
  database: string;
}
