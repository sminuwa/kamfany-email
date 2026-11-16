import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  fetchNotices,
  markNotice,
  markNotices,
  type DeskNotice,
} from "../services/notice.service";

export const useNoticeStore = defineStore("notices", () => {
  const items = ref<DeskNotice[]>([]);
  const unread = computed(() => items.value.filter((item) => !item.readAt).length);

  async function load() {
    items.value = await fetchNotices();
  }

  function receive(notice: DeskNotice) {
    if (items.value.some((item) => item.id === notice.id)) {
      return;
    }
    items.value = [notice, ...items.value].slice(0, 30);
  }

  async function mark(id: string) {
    await markNotice(id);
    const item = items.value.find((notice) => notice.id === id);
    if (item) {
      item.readAt = new Date().toISOString();
    }
  }

  async function markAll() {
    await markNotices();
    const now = new Date().toISOString();
    for (const item of items.value) {
      item.readAt = item.readAt ?? now;
    }
  }

  return { items, unread, load, receive, mark, markAll };
});
