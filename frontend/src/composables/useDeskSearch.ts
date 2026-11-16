import { watch } from "vue";
import { useMailStore } from "../stores/messages";
import { useUiStore } from "../stores/ui";

export function useDeskSearch() {
  const ui = useUiStore();
  const mail = useMailStore();
  let timer = 0;

  watch(
    () =>
      [
        ui.searchQuery,
        mail.mailboxId ?? "",
        mail.search.from,
        mail.search.to,
        mail.search.subject,
        mail.search.after,
        mail.search.before,
        mail.search.folder,
        mail.search.labelId,
        String(mail.search.allMailboxes),
        String(mail.filters.unread),
        String(mail.filters.attachments),
        String(mail.filters.important),
      ].join("|"),
    () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        void mail.applySearch(ui.searchQuery);
      }, 300);
    },
  );
}
