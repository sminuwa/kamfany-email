<script setup lang="ts">
import { onMounted, ref } from "vue";
import { fetchProviderStatus, syncProvider, type ProviderStatus } from "../../services/provider.service";
import { errorMessage } from "../../services/http";

const provider = ref<ProviderStatus | null>(null);
const notice = ref("");

onMounted(async () => {
  try {
    provider.value = await fetchProviderStatus();
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

async function check() {
  notice.value = "";
  try {
    provider.value = await syncProvider();
    notice.value = provider.value.configured
      ? `${provider.value.sendingDomain} is ${provider.value.domainStatus}. Receiving is ${provider.value.receiving}.`
      : "Resend is not configured on the server.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}
</script>

<template>
  <section>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Email provider</h1>
        <p v-if="provider" class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
          Sending goes through Resend for @{{ provider.sendingDomain }}. The compose screen sends through this
          provider. Checking with Resend does not create a receiving address. A mailbox stays unverified until
          Resend lists that inbox or a message arrives for it.
        </p>
      </div>
      <button v-if="provider" type="button" class="km-action" @click="check">Check with Resend</button>
    </div>
    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>
    <div v-if="provider" class="mt-6 rounded-panel border border-line bg-surface px-4">
      <dl class="divide-y divide-line">
        <div class="grid gap-1 py-3 sm:grid-cols-[14rem_minmax(0,1fr)] sm:items-baseline">
          <dt class="text-sm text-muted">API key</dt>
          <dd class="text-sm">{{ provider.configured ? "The API key is set on the server." : "Resend is not configured on the server." }}</dd>
        </div>
        <div class="grid gap-1 py-3 sm:grid-cols-[14rem_minmax(0,1fr)] sm:items-baseline">
          <dt class="text-sm text-muted">Webhook signing</dt>
          <dd class="text-sm">
            {{
              provider.webhookConfigured
                ? "Webhook signing is set."
                : "Webhook signing is not set, so inbound events are refused."
            }}
          </dd>
        </div>
        <div class="grid gap-1 py-3 sm:grid-cols-[14rem_minmax(0,1fr)] sm:items-baseline">
          <dt class="text-sm text-muted">Webhook queue</dt>
          <dd class="text-sm">
            {{ provider.queueReceived }} waiting, {{ provider.queueProcessing }} processing,
            {{ provider.queueFailed }} failed.
            <span v-if="provider.queueOldestSeconds !== null">
              The oldest waiting event is {{ provider.queueOldestSeconds }} seconds old.
            </span>
          </dd>
        </div>
        <div class="grid gap-1 py-3 sm:grid-cols-[14rem_minmax(0,1fr)] sm:items-baseline">
          <dt class="text-sm text-muted">Malware scanning</dt>
          <dd class="text-sm">
            {{
              provider.scanner === "clamav"
                ? "ClamAV is configured."
                : "Malware scanning is not configured. Attachments stay marked not scanned."
            }}
          </dd>
        </div>
        <div v-if="provider.domainStatus !== 'not_checked'" class="grid gap-1 py-3 sm:grid-cols-[14rem_minmax(0,1fr)] sm:items-baseline">
          <dt class="text-sm text-muted">Domain</dt>
          <dd class="text-sm">{{ provider.sendingDomain }} is {{ provider.domainStatus }}. Receiving is {{ provider.receiving }}.</dd>
        </div>
      </dl>
    </div>
  </section>
</template>
