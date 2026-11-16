import type { ResolvedTheme } from "./theme";

const THEME_COLORS = {
  light: {
    background: "#f4f8f8",
    ink: "#10211f",
    muted: "#3e5552",
    link: "#0e3b3a",
  },
  dark: {
    background: "#141d1c",
    ink: "#e7f3f1",
    muted: "#9bb5b1",
    link: "#f2c14e",
  },
} as const;

export function emailSrcdoc(html: string, theme: ResolvedTheme): string {
  const colors = THEME_COLORS[theme];
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'"><style>
    body { margin: 0; padding: 8px 20px 32px; font: 15px/1.55 "Avenir Next", "Segoe UI", sans-serif; color: ${colors.ink}; background: ${colors.background}; overflow-wrap: anywhere; }
    a { color: ${colors.link}; }
    img { max-width: 100%; height: auto; }
    p { margin: 0 0 0.9em; }
    blockquote { margin: 0 0 0.9em; padding-left: 0.9em; border-left: 3px solid ${colors.muted}; color: ${colors.muted}; }
    pre { white-space: pre-wrap; font: inherit; }
  </style></head><body>${html}</body></html>`;
}
