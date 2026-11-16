<script setup lang="ts">
import { ChevronLeft, ChevronRight } from "@lucide/vue";
import { computed } from "vue";
import { PAGE_SIZE } from "../../utils/pager";

const page = defineModel<number>({ required: true });

const props = defineProps<{
  pages: number;
  total: number;
  pageSize?: number;
}>();

const size = computed(() => props.pageSize ?? PAGE_SIZE);
const from = computed(() => (props.total === 0 ? 0 : (page.value - 1) * size.value + 1));
const to = computed(() => Math.min(page.value * size.value, props.total));
</script>

<template>
  <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
    <p class="text-sm text-muted">Showing {{ from }}–{{ to }} of {{ total }}</p>
    <div v-if="pages > 1" class="flex items-center gap-2">
      <button
        type="button"
        class="grid size-8 place-items-center rounded-control border border-line bg-surface text-ink hover:bg-canvas disabled:opacity-40"
        :disabled="page <= 1"
        aria-label="Previous page"
        @click="page = Math.max(1, page - 1)"
      >
        <ChevronLeft class="size-4" />
      </button>
      <span class="km-page">{{ page }}</span>
      <button
        type="button"
        class="grid size-8 place-items-center rounded-control border border-line bg-surface text-ink hover:bg-canvas disabled:opacity-40"
        :disabled="page >= pages"
        aria-label="Next page"
        @click="page = Math.min(pages, page + 1)"
      >
        <ChevronRight class="size-4" />
      </button>
    </div>
  </div>
</template>
