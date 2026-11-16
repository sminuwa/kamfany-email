import { describe, expect, it } from "vitest";
import { deliveryLabel, htmlToText, textToHtml, withSignature } from "./message-html";

describe("message html", () => {
  it("turns plain text into paragraphs and back", () => {
    const html = textToHtml("Hello\n\nthere");
    expect(html).toContain("<p>Hello</p>");
    expect(htmlToText(html)).toBe("Hello\nthere");
  });

  it("appends a signature and names delivery states", () => {
    expect(withSignature("<p>Note</p>", "<p>Adaeze</p>")).toContain("Adaeze");
    expect(deliveryLabel("accepted")).toBe("Accepted by the provider");
    expect(deliveryLabel("received")).toBeNull();
  });
});
