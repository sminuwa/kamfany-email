export function textToHtml(value: string): string {
  const blocks = value.split(/\n{2,}/);
  return blocks
    .map((block) => `<p>${escapeHtml(block).replaceAll("\n", "<br>")}</p>`)
    .join("");
}

export function htmlToText(value: string): string {
  const withBreaks = value
    .replaceAll(/<br\s*\/?>/gi, "\n")
    .replaceAll(/<\/(p|div|li|h1|h2|h3|blockquote)>/gi, "\n");
  const text = withBreaks.replaceAll(/<[^>]+>/g, "");
  return decode(text).replaceAll(/\n{3,}/g, "\n\n").trim();
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function decode(value: string): string {
  return value
    .replaceAll("&nbsp;", " ")
    .replaceAll("&amp;", "&")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", "\"");
}

export function deliveryLabel(status: string | undefined): string | null {
  switch (status) {
    case "sending":
      return "Sending";
    case "accepted":
      return "Accepted by the provider";
    case "delivered":
      return "Delivered";
    case "bounced":
      return "Bounced";
    case "failed":
      return "Failed";
    case "draft":
      return "Draft";
    default:
      return null;
  }
}

export function withSignature(html: string, signature: string): string {
  const body = html.trim();
  const sign = signature.trim();
  if (!sign) {
    return body;
  }
  if (!body) {
    return sign;
  }
  return `${body}<p></p>${sign}`;
}
