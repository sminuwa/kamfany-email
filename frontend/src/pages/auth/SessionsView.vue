<script setup lang="ts">
import dayjs from "dayjs";
import { computed, onMounted, ref, watch } from "vue";
import ListPager from "../../components/common/ListPager.vue";
import { type DeskSession } from "../../services/auth.service";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";
import { browserLabel } from "../../utils/account-labels";
import { pageCount, pageSlice } from "../../utils/pager";

const auth = useAuthStore();
const sessions = ref<DeskSession[]>([]);
const notice = ref("");
const page = ref(1);
const pages = computed(() => pageCount(sessions.value.length));
const visible = computed(() => pageSlice(sessions.value, page.value));

watch(pages, (count) => {
  if (page.value > count) {
    page.value = count;
  }
});

onMounted(async () => {
  try {
    sessions.value = await auth.sessions();
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

async function endSession(session: DeskSession) {
  notice.value = "";
  try {
    await auth.endSession(session.id);
    if (session.current) {
      await auth.logout();
      return;
    }
    sessions.value = await auth.sessions();
    notice.value = "Session ended.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}
</script>

<template>
  <section>
    <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Sessions</h1>
    <p class="mt-3 max-w-[48ch] text-sm leading-relaxed text-muted">
      Browsers that are signed in to this account. Ending this session signs you out.
    </p>
    <p v-if="notice" class="mt-6 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>
    <p v-if="sessions.length === 0" class="mt-6 text-sm text-muted">No browser is signed in.</p>
    <ul v-else class="km-card mt-6 divide-y divide-line overflow-hidden">
      <li v-for="session in visible" :key="session.id" class="km-row flex items-center gap-3 px-4 py-3">
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm" :title="session.userAgent || undefined">
            {{ browserLabel(session.userAgent) }}
          </p>
          <p class="text-xs text-muted">
            Started {{ dayjs(session.createdAt).format("D MMM YYYY, HH:mm") }}
          </p>
          <span v-if="session.current" class="km-pill mint mt-1">This session</span>
        </div>
        <button type="button" class="km-mini line" @click="endSession(session)">End</button>
      </li>
    </ul>
    <ListPager v-if="sessions.length" v-model="page" :pages="pages" :total="sessions.length" />
  </section>
</template>
