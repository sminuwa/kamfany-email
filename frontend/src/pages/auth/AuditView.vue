<script setup lang="ts">
import dayjs from "dayjs";
import { computed, onMounted, ref, watch } from "vue";
import ListPager from "../../components/common/ListPager.vue";
import { fetchAudit, type AuditEntry } from "../../services/audit.service";
import { errorMessage } from "../../services/http";
import { auditActionLabel } from "../../utils/account-labels";
import { pageCount, pageSlice } from "../../utils/pager";

const audit = ref<AuditEntry[]>([]);
const notice = ref("");
const page = ref(1);
const pages = computed(() => pageCount(audit.value.length));
const visible = computed(() => pageSlice(audit.value, page.value));

watch(pages, (count) => {
  if (page.value > count) {
    page.value = count;
  }
});

onMounted(async () => {
  try {
    audit.value = await fetchAudit();
  } catch (error) {
    notice.value = errorMessage(error);
  }
});
</script>

<template>
  <section>
    <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Audit log</h1>
    <p class="mt-3 max-w-[62ch] text-sm leading-relaxed text-muted">Recent actions recorded for this desk.</p>
    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm text-danger" role="alert">{{ notice }}</p>
    <p v-else-if="audit.length === 0" class="mt-6 text-sm text-muted">No audit records yet.</p>
    <div v-else class="mt-6 overflow-x-auto rounded-panel border border-line bg-surface">
      <table class="km-table">
        <thead>
          <tr>
            <th>Action</th>
            <th>Person</th>
            <th>When</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in visible" :key="entry.id">
            <td class="font-medium">{{ auditActionLabel(entry.action) }}</td>
            <td class="text-muted">{{ entry.actorEmail || "Unknown person" }}</td>
            <td class="text-muted">{{ dayjs(entry.createdAt).format("D MMM YYYY, HH:mm") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <ListPager v-if="audit.length" v-model="page" :pages="pages" :total="audit.length" />
  </section>
</template>
