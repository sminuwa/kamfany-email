import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { fetchHealth } from "../services/health.service";

export type ServiceStatus = "unknown" | "ready" | "degraded" | "offline";

export const useServiceStore = defineStore("service", () => {
  const status = ref<ServiceStatus>("unknown");
  const checking = ref(false);

  const label = computed(() => {
    switch (status.value) {
      case "ready":
        return "Mail service is ready.";
      case "degraded":
        return "Mail service is up. Database is not connected.";
      case "offline":
        return "Mail service is not reachable.";
      default:
        return "Checking the mail service.";
    }
  });

  async function refresh() {
    if (checking.value) {
      return;
    }
    checking.value = true;
    try {
      const report = await fetchHealth();
      status.value = report.database === "connected" ? "ready" : "degraded";
    } catch {
      status.value = "offline";
    } finally {
      checking.value = false;
    }
  }

  return { status, checking, label, refresh };
});
