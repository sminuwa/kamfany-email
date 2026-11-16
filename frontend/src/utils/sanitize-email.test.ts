import { describe, expect, it } from "vitest";
import { loadSampleMailbox } from "../services/message.service";
import { sanitizeEmailHtml } from "./sanitize-email";

describe("sanitize email html", () => {
  it("drops scripts, event handlers, javascript links, and remote images", () => {
    const clean = sanitizeEmailHtml(`
      <p onclick="steal()">Hello</p>
      <script>window.stolen = document.cookie</script>
      <style>body { background: url("https://evil.example/x") }</style>
      <img src="https://tracking.example/pixel.gif" alt="">
      <a href="javascript:alert(1)">Reset</a>
      <a href="https://kamfany.com/security">Security page</a>
      <img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" alt="mark">
    `);
    const lower = clean.toLowerCase();
    expect(lower).not.toContain("<script");
    expect(lower).not.toContain("onclick");
    expect(lower).not.toContain("evil.example");
    expect(lower).not.toContain("tracking.example");
    expect(lower).not.toContain("javascript:");
    expect(clean).toContain("Hello");
    expect(clean).toContain("Reset");
    expect(clean).toContain('href="https://kamfany.com/security"');
    expect(clean).toContain("data:image/gif;base64,");
  });

  it("cleans the sample sign-in message", () => {
    const message = loadSampleMailbox().messages.find((item) => item.id === "m07");
    const clean = sanitizeEmailHtml(message?.bodyHtml ?? "");
    expect(clean.toLowerCase()).not.toContain("<script");
    expect(clean.toLowerCase()).not.toContain("onclick");
    expect(clean).not.toContain("tracking.example");
    expect(clean).not.toContain("javascript:");
    expect(clean).toContain("Do not send a password");
    expect(clean).toContain('href="https://kamfany.com/security"');
  });
});
