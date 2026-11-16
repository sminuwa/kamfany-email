const ALLOWED_TAGS = new Set([
  "a",
  "p",
  "br",
  "strong",
  "b",
  "em",
  "i",
  "u",
  "ul",
  "ol",
  "li",
  "blockquote",
  "h1",
  "h2",
  "h3",
  "h4",
  "span",
  "div",
  "pre",
  "code",
  "table",
  "thead",
  "tbody",
  "tr",
  "th",
  "td",
  "hr",
]);

const SKIP_CONTENTS = new Set(["script", "style", "iframe", "object", "embed"]);

const DATA_IMAGE =
  /^data:image\/(png|jpeg|gif|webp);base64,[a-z0-9+/=]+$/i;

function safeHref(value: string | null): string | null {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  if (!/^(https?:|mailto:)/i.test(trimmed)) {
    return null;
  }
  try {
    const url = new URL(trimmed);
    if (url.protocol === "http:" || url.protocol === "https:" || url.protocol === "mailto:") {
      return url.href;
    }
  } catch {
    return null;
  }
  return null;
}

function safeImage(value: string | null): string | null {
  if (!value) {
    return null;
  }
  const compact = value.trim().replace(/\s/g, "");
  return DATA_IMAGE.test(compact) ? compact : null;
}

function safeSpan(value: string | null): string | null {
  if (value && /^\d{1,2}$/.test(value)) {
    return value;
  }
  return null;
}

function appendChildren(source: Node, target: HTMLElement) {
  for (const node of source.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      target.append(node.textContent ?? "");
      continue;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) {
      continue;
    }
    const element = node as HTMLElement;
    const tag = element.tagName.toLowerCase();
    if (SKIP_CONTENTS.has(tag)) {
      continue;
    }
    if (tag === "img") {
      const src = safeImage(element.getAttribute("src"));
      if (!src) {
        continue;
      }
      const image = document.createElement("img");
      image.setAttribute("src", src);
      image.setAttribute("alt", element.getAttribute("alt") ?? "");
      target.append(image);
      continue;
    }
    if (!ALLOWED_TAGS.has(tag)) {
      appendChildren(element, target);
      continue;
    }
    if (tag === "a") {
      const href = safeHref(element.getAttribute("href"));
      if (!href) {
        appendChildren(element, target);
        continue;
      }
      const link = document.createElement("a");
      link.setAttribute("href", href);
      appendChildren(element, link);
      target.append(link);
      continue;
    }
    const next = document.createElement(tag);
    if (tag === "td" || tag === "th") {
      const colspan = safeSpan(element.getAttribute("colspan"));
      const rowspan = safeSpan(element.getAttribute("rowspan"));
      if (colspan) {
        next.setAttribute("colspan", colspan);
      }
      if (rowspan) {
        next.setAttribute("rowspan", rowspan);
      }
    }
    appendChildren(element, next);
    target.append(next);
  }
}

export function sanitizeEmailHtml(unsafe: string): string {
  const parsed = new DOMParser().parseFromString(unsafe, "text/html");
  const clean = document.createElement("div");
  appendChildren(parsed.body, clean);
  return clean.innerHTML;
}
