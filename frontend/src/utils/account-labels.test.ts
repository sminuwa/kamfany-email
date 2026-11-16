import { describe, expect, it } from "vitest";
import { auditActionLabel, browserLabel } from "./account-labels";

describe("account labels", () => {
  it("names known audit actions", () => {
    expect(auditActionLabel("auth.login")).toBe("Signed in");
    expect(auditActionLabel("mailbox.updated")).toBe("Mailbox updated");
    expect(auditActionLabel("custom.event")).toBe("custom.event");
  });

  it("names the browser without the raw agent string", () => {
    expect(
      browserLabel(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
      ),
    ).toBe("Chrome on Mac");
    expect(browserLabel(null)).toBe("Unknown browser");
  });
});
