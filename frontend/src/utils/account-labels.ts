const AUDIT_ACTIONS: Record<string, string> = {
  "auth.login": "Signed in",
  "auth.login_failed": "Sign-in failed",
  "auth.logout": "Signed out",
  "auth.password_changed": "Password changed",
  "auth.password_reset": "Password reset",
  "auth.session_revoked": "Session ended",
  "user.created": "Account created",
  "mailbox.created": "Mailbox registered",
  "mailbox.updated": "Mailbox updated",
  "mailbox.deleted": "Mailbox removed",
  "mailbox.member_granted": "Mailbox access granted",
  "mailbox.member_revoked": "Mailbox access removed",
  "message.deleted": "Message deleted",
  "contact.deleted": "Contact deleted",
  "mail.accepted": "Message accepted for sending",
  "mail.failed": "Message failed to send",
};

export function auditActionLabel(action: string): string {
  return AUDIT_ACTIONS[action] ?? action;
}

export function browserLabel(agent: string | null): string {
  if (!agent) {
    return "Unknown browser";
  }
  const browser = agent.includes("Edg/")
    ? "Edge"
    : agent.includes("Chrome/") || agent.includes("HeadlessChrome")
      ? "Chrome"
      : agent.includes("Firefox/")
        ? "Firefox"
        : agent.includes("Safari/")
          ? "Safari"
          : "Browser";
  const device = agent.includes("iPhone")
    ? "iPhone"
    : agent.includes("iPad")
      ? "iPad"
      : agent.includes("Android")
        ? "Android"
        : agent.includes("Mac OS")
          ? "Mac"
          : agent.includes("Windows")
            ? "Windows"
            : agent.includes("Linux")
              ? "Linux"
              : "";
  return device ? `${browser} on ${device}` : browser;
}
