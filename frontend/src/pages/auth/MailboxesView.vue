<script setup lang="ts">
import { ArrowUpDown, Clock, CircleCheck, CircleOff, Ellipsis, ListFilter, Mail, Plus } from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import GrantAccessDialog from "../../components/account/GrantAccessDialog.vue";
import RegisterMailboxDialog from "../../components/account/RegisterMailboxDialog.vue";
import ListPager from "../../components/common/ListPager.vue";
import MenuPanel from "../../components/common/MenuPanel.vue";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";
import { useMailboxStore } from "../../stores/mailboxes";
import { selectMailboxes, type MailboxSort, type MailboxStatus } from "../../utils/mailbox-search";
import { pageCount, pageSlice } from "../../utils/pager";
import { field, quiet } from "./account-form";

const auth = useAuthStore();
const boxes = useMailboxStore();
const route = useRoute();
const router = useRouter();
const notice = ref("");
const registerOpen = ref(false);
const grantOpen = ref(false);
const grantMailboxId = ref("");
const query = ref("");
const status = ref<MailboxStatus>("all");
const sort = ref<MailboxSort>("newest");
const page = ref(1);
const menuFor = ref<string | null>(null);

const matched = computed(() => selectMailboxes(boxes.items, query.value, status.value, sort.value));
const pages = computed(() => pageCount(matched.value.length));
const visible = computed(() => pageSlice(matched.value, page.value));
const enabledCount = computed(() => boxes.items.filter((mailbox) => mailbox.isEnabled).length);
const verifiedCount = computed(() => boxes.items.filter((mailbox) => mailbox.isReceivingVerified).length);

watch([query, status, sort], () => {
  page.value = 1;
});

watch(pages, (count) => {
  if (page.value > count) {
    page.value = count;
  }
});

onMounted(async () => {
  try {
    await boxes.ensure();
    openGrantFromQuery();
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

watch(
  () => route.query.mailbox,
  () => {
    if (boxes.loaded) {
      openGrantFromQuery();
    }
  },
);

function openGrant(id: string) {
  grantMailboxId.value = id;
  grantOpen.value = true;
  menuFor.value = null;
}

function openGrantFromQuery() {
  const requested = typeof route.query.mailbox === "string" ? route.query.mailbox : "";
  if (!requested) {
    return;
  }
  const mailbox = boxes.items.find((item) => item.id === requested && item.canManage);
  if (mailbox) {
    openGrant(mailbox.id);
  }
  void router.replace({ name: "account-mailboxes" });
}

async function toggleEnabled(id: string, isEnabled: boolean) {
  notice.value = "";
  try {
    await boxes.setEnabled(id, isEnabled);
    notice.value = isEnabled
      ? "Mailbox enabled. Mail is still not received until the provider route is confirmed."
      : "Mailbox disabled.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function useMailbox(id: string) {
  await boxes.setDefault(id);
  boxes.select(id);
  await router.push("/");
}

const stats = computed(() => [
  { label: "Total mailboxes", value: String(boxes.items.length), icon: Mail, tone: "mint" },
  { label: "Enabled", value: String(enabledCount.value), icon: CircleCheck, tone: "mint" },
  { label: "Disabled", value: String(boxes.items.length - enabledCount.value), icon: CircleOff, tone: "rose" },
  {
    label: "Receiving verification",
    value: `${verifiedCount.value} / ${boxes.items.length}`,
    icon: Clock,
    tone: "sand",
  },
]);
</script>

<template>
  <section>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Mailboxes</h1>
        <p class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
          Enabling a mailbox does not start receiving. A mailbox stays unverified until its provider route is confirmed.
        </p>
      </div>
      <button v-if="auth.canManageMailboxes" type="button" class="km-action" @click="registerOpen = true">
        <Plus class="size-4" />
        Register a mailbox
      </button>
    </div>

    <RegisterMailboxDialog
      v-model="registerOpen"
      @registered="
        notice = $event;
        page = 1;
      "
    />
    <GrantAccessDialog v-model="grantOpen" :mailbox-id="grantMailboxId" @granted="notice = $event" />

    <div class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <article v-for="stat in stats" :key="stat.label" class="rounded-panel border border-line bg-surface px-4 py-4">
        <div class="flex items-center gap-3">
          <span class="km-badge" :class="stat.tone">
            <component :is="stat.icon" class="size-4" />
          </span>
          <div>
            <p class="text-sm text-muted">{{ stat.label }}</p>
            <p class="text-2xl font-semibold tabular-nums tracking-[-0.03em]">{{ stat.value }}</p>
          </div>
        </div>
      </article>
    </div>

    <div class="mt-4 grid gap-2 min-[900px]:grid-cols-[minmax(0,1fr)_11.5rem_11.5rem]">
      <label class="text-sm">
        <span class="sr-only">Search mailboxes</span>
        <input v-model="query" type="search" placeholder="Search mailboxes (address or name)" :class="field" />
      </label>
      <label class="relative block">
        <ListFilter class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted" />
        <select v-model="status" :class="[field, 'pl-9']" aria-label="Status">
          <option value="all">All statuses</option>
          <option value="enabled">Enabled</option>
          <option value="disabled">Disabled</option>
        </select>
      </label>
      <label class="relative block">
        <ArrowUpDown class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted" />
        <select v-model="sort" :class="[field, 'pl-9']" aria-label="Sort">
          <option value="newest">Newest first</option>
          <option value="oldest">Oldest first</option>
          <option value="address">Address</option>
        </select>
      </label>
    </div>

    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <ul v-if="visible.length" class="mt-4 divide-y divide-line rounded-panel border border-line bg-surface min-[800px]:hidden">
      <li v-for="mailbox in visible" :key="mailbox.id" class="space-y-3 px-4 py-3">
        <div>
          <p class="flex items-center gap-2 font-medium">
            <Mail class="size-4 shrink-0 text-muted" />
            <span class="truncate">{{ mailbox.address }}</span>
          </p>
          <p class="mt-1 text-sm text-muted">{{ mailbox.displayName }}</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <span class="km-pill" :class="mailbox.isEnabled ? 'mint' : 'rose'">
            <span class="size-1.5 rounded-full bg-current" />
            {{ mailbox.isEnabled ? "Enabled" : "Disabled" }}
          </span>
          <span class="inline-flex items-center gap-1.5 text-sm text-muted">
            <span class="size-1.5 rounded-full" :class="mailbox.isReceivingVerified ? 'bg-status-ready' : 'bg-muted'" />
            {{ mailbox.isReceivingVerified ? "Verified" : "Not verified" }}
          </span>
        </div>
        <div class="flex items-center gap-1">
          <button v-if="mailbox.canRead" type="button" :class="quiet" @click="useMailbox(mailbox.id)">Open</button>
          <button
            v-if="mailbox.canManage"
            type="button"
            class="km-mini"
            :class="mailbox.isEnabled ? 'line' : 'mint'"
            @click="toggleEnabled(mailbox.id, !mailbox.isEnabled)"
          >
            {{ mailbox.isEnabled ? "Disable" : "Enable" }}
          </button>
          <MenuPanel
            v-if="mailbox.canManage"
            :model-value="menuFor === mailbox.id"
            label="Mailbox actions"
            @update:model-value="menuFor = $event ? mailbox.id : null"
          >
            <template #trigger="{ toggle, open }">
              <button
                type="button"
                class="grid size-9 place-items-center rounded-control text-muted hover:bg-canvas hover:text-ink"
                :aria-expanded="open"
                aria-label="More mailbox actions"
                @click="toggle"
              >
                <Ellipsis class="size-4" />
              </button>
            </template>
            <button
              type="button"
              role="menuitem"
              class="block w-full rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
              @click="openGrant(mailbox.id)"
            >
              Grant access
            </button>
          </MenuPanel>
        </div>
      </li>
    </ul>

    <div v-if="visible.length" class="mt-4 hidden rounded-panel border border-line bg-surface min-[800px]:block">
      <table class="km-table min-w-[720px]">
        <thead>
          <tr>
            <th>Mailbox address</th>
            <th>Display name</th>
            <th>Status</th>
            <th>Receiving verification</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="mailbox in visible" :key="mailbox.id">
            <td>
              <span class="flex items-center gap-2 font-medium">
                <Mail class="size-4 shrink-0 text-muted" />
                <span class="truncate">{{ mailbox.address }}</span>
              </span>
            </td>
            <td class="text-muted">{{ mailbox.displayName }}</td>
            <td>
              <span class="km-pill" :class="mailbox.isEnabled ? 'mint' : 'rose'">
                <span class="size-1.5 rounded-full bg-current" />
                {{ mailbox.isEnabled ? "Enabled" : "Disabled" }}
              </span>
            </td>
            <td>
              <span class="inline-flex items-center gap-1.5 text-muted">
                <span
                  class="size-1.5 rounded-full"
                  :class="mailbox.isReceivingVerified ? 'bg-status-ready' : 'bg-muted'"
                />
                {{ mailbox.isReceivingVerified ? "Verified" : "Not verified" }}
              </span>
            </td>
            <td>
              <div class="flex items-center gap-1">
                <button v-if="mailbox.canRead" type="button" :class="quiet" @click="useMailbox(mailbox.id)">Open</button>
                <button
                  v-if="mailbox.canManage"
                  type="button"
                  class="km-mini"
                  :class="mailbox.isEnabled ? 'line' : 'mint'"
                  @click="toggleEnabled(mailbox.id, !mailbox.isEnabled)"
                >
                  {{ mailbox.isEnabled ? "Disable" : "Enable" }}
                </button>
                <MenuPanel
                  v-if="mailbox.canManage"
                  :model-value="menuFor === mailbox.id"
                  label="Mailbox actions"
                  @update:model-value="menuFor = $event ? mailbox.id : null"
                >
                  <template #trigger="{ toggle, open }">
                    <button
                      type="button"
                      class="grid size-9 place-items-center rounded-control text-muted hover:bg-canvas hover:text-ink"
                      :aria-expanded="open"
                      aria-label="More mailbox actions"
                      @click="toggle"
                    >
                      <Ellipsis class="size-4" />
                    </button>
                  </template>
                  <button
                    type="button"
                    role="menuitem"
                    class="block w-full rounded-[8px] px-2.5 py-2 text-left text-sm hover:bg-canvas"
                    @click="openGrant(mailbox.id)"
                  >
                    Grant access
                  </button>
                </MenuPanel>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else class="mt-6 text-sm text-muted">
      {{ query.trim() || status !== "all" ? "Nothing matches that search." : "No mailbox is available to this account." }}
    </p>
    <ListPager v-model="page" :pages="pages" :total="matched.length" />
  </section>
</template>
