import { AxiosError } from "axios";
import { describe, expect, it } from "vitest";
import { errorMessage } from "./http";

describe("errorMessage", () => {
  it("names a network failure", () => {
    const error = new AxiosError("Network Error", "ERR_NETWORK");
    expect(errorMessage(error)).toBe("Mail service is not reachable.");
  });

  it("uses the API error message when the service sent one", () => {
    const error = new AxiosError("Request failed", "ERR_BAD_RESPONSE");
    error.response = {
      data: { error: { code: "not_ready", message: "Database is unavailable." } },
      status: 503,
      statusText: "Service Unavailable",
      headers: {},
      config: error.config ?? { headers: {} as never },
    };
    expect(errorMessage(error)).toBe("Database is unavailable.");
  });

  it("falls back when the failure is unknown", () => {
    expect(errorMessage(new Error("nope"))).toBe("Something went wrong. Try again.");
  });
});
