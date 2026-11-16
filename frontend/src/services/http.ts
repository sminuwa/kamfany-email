import axios from "axios";

export const http = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL ?? "",
  timeout: 15_000,
  withCredentials: true,
  headers: {
    Accept: "application/json",
    "X-Kamfany-Request": "1",
  },
});

export function errorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const message = error.response?.data?.error?.message;
    if (typeof message === "string" && message.length > 0) {
      return message;
    }
    if (error.code === "ERR_NETWORK" || error.code === "ECONNABORTED") {
      return "Mail service is not reachable.";
    }
  }
  return "Something went wrong. Try again.";
}

http.interceptors.response.use(
  (response) => response,
  (error: unknown) => {
    if (axios.isAxiosError(error)) {
      error.message = errorMessage(error);
    }
    return Promise.reject(error);
  },
);
