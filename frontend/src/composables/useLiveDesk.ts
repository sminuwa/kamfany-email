import { onBeforeUnmount, onMounted } from "vue";
import { useMailStore } from "../stores/messages";
import { useNoticeStore } from "../stores/notices";
import type { DeskNotice } from "../services/notice.service";

export function useLiveDesk() {
  const mail = useMailStore();
  const notices = useNoticeStore();
  let socket: WebSocket | null = null;
  let timer = 0;
  let stopped = false;

  function connect() {
    if (stopped) {
      return;
    }
    const protocol = window.location.protocol === "https:" ? "wss" : "ws";
    socket = new WebSocket(`${protocol}://${window.location.host}/api/v1/ws`);
    socket.addEventListener("message", (event) => {
      if (typeof event.data !== "string") {
        return;
      }
      const payload = parseEvent(event.data);
      if (!payload) {
        return;
      }
      notices.receive(payload.notice);
      mail.noteArrival(payload.mailboxId, payload.inboxUnread);
    });
    socket.addEventListener("close", () => {
      if (!stopped) {
        timer = window.setTimeout(connect, 3000);
      }
    });
  }

  onMounted(() => {
    void notices.load().catch(() => {
      // The bell stays empty until the next connection.
    });
    connect();
  });

  onBeforeUnmount(() => {
    stopped = true;
    window.clearTimeout(timer);
    socket?.close();
  });
}

function parseEvent(raw: string): { notice: DeskNotice; mailboxId: string; inboxUnread: number } | null {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!value || typeof value !== "object") {
    return null;
  }
  const event = value as Record<string, unknown>;
  if (event.type !== "mail.received") {
    return null;
  }
  if (typeof event.notification_id !== "string" || typeof event.mailbox_id !== "string") {
    return null;
  }
  return {
    mailboxId: event.mailbox_id,
    inboxUnread: typeof event.inbox_unread === "number" ? event.inbox_unread : 0,
    notice: {
      id: event.notification_id,
      mailboxId: event.mailbox_id,
      messageId: typeof event.message_id === "string" ? event.message_id : null,
      title: typeof event.title === "string" ? event.title : "New mail",
      body: typeof event.body === "string" ? event.body : "",
      readAt: null,
      createdAt: new Date().toISOString(),
    },
  };
}
