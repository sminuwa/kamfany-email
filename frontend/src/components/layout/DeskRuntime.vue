<script setup lang="ts">
import { onMounted } from "vue";
import { useDeskSearch } from "../../composables/useDeskSearch";
import { useLiveDesk } from "../../composables/useLiveDesk";
import { useServiceStore } from "../../stores/service";

useDeskSearch();
useLiveDesk();

const service = useServiceStore();

onMounted(() => {
  void service.refresh();
});
</script>

<template>
  <span class="hidden" />
</template>
