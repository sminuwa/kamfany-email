<script setup lang="ts">
import { ArrowUpDown, ListFilter, Plus, Shield, ShieldCheck, User, Users } from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import CreateAccountDialog from "../../components/account/CreateAccountDialog.vue";
import ListPager from "../../components/common/ListPager.vue";
import { type PersonAccount } from "../../services/auth.service";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";
import { PAGE_SIZE, pageCount, pageSlice } from "../../utils/pager";
import { selectPeople, type PersonRole, type PersonSort } from "../../utils/people-search";
import { field } from "./account-form";

const auth = useAuthStore();
const people = ref<PersonAccount[]>([]);
const notice = ref("");
const createOpen = ref(false);
const query = ref("");
const role = ref<PersonRole>("all");
const sort = ref<PersonSort>("newest");
const page = ref(1);

const matched = computed(() => selectPeople(people.value, query.value, role.value, sort.value));
const pages = computed(() => pageCount(matched.value.length));
const visible = computed(() => pageSlice(matched.value, page.value));
const ownerCount = computed(() => people.value.filter((person) => person.roles.includes("owner")).length);
const administratorCount = computed(() => people.value.filter((person) => person.roles.includes("administrator")).length);
const memberCount = computed(() => people.value.filter((person) => person.roles.includes("member")).length);

watch([query, role, sort], () => {
  page.value = 1;
});

watch(pages, (count) => {
  if (page.value > count) {
    page.value = count;
  }
});

onMounted(async () => {
  try {
    people.value = await auth.people();
  } catch (error) {
    notice.value = errorMessage(error);
  }
});

async function onCreated(email: string) {
  notice.value = "Account created.";
  try {
    people.value = await auth.people();
    const index = matched.value.findIndex((person) => person.email.toLowerCase() === email.toLowerCase());
    page.value = index >= 0 ? Math.floor(index / PAGE_SIZE) + 1 : 1;
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

function roleLabel(name: string): string {
  if (name === "owner") {
    return "Owner";
  }
  if (name === "administrator") {
    return "Administrator";
  }
  if (name === "member") {
    return "Member";
  }
  return name;
}

function roleTone(name: string): string {
  if (name === "owner") {
    return "mint";
  }
  if (name === "administrator") {
    return "sand";
  }
  return "quiet";
}

const stats = computed(() => [
  { label: "Total accounts", value: String(people.value.length), icon: Users, tone: "mint" },
  { label: "Owners", value: String(ownerCount.value), icon: Shield, tone: "mint" },
  { label: "Administrators", value: String(administratorCount.value), icon: ShieldCheck, tone: "sand" },
  { label: "Members", value: String(memberCount.value), icon: User, tone: "quiet" },
]);
</script>

<template>
  <section>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">People</h1>
        <p class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
          Accounts that can sign in to this desk. An owner or an administrator can add a member or an administrator.
          The owner role stays with the first account.
        </p>
      </div>
      <button v-if="auth.canManageUsers" type="button" class="km-action" @click="createOpen = true">
        <Plus class="size-4" />
        Create an account
      </button>
    </div>

    <CreateAccountDialog v-model="createOpen" @created="onCreated" />

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
        <span class="sr-only">Search people</span>
        <input v-model="query" type="search" placeholder="Search people (email or name)" :class="field" />
      </label>
      <label class="relative block">
        <ListFilter class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted" />
        <select v-model="role" :class="[field, 'pl-9']" aria-label="Role">
          <option value="all">All roles</option>
          <option value="owner">Owner</option>
          <option value="administrator">Administrator</option>
          <option value="member">Member</option>
        </select>
      </label>
      <label class="relative block">
        <ArrowUpDown class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted" />
        <select v-model="sort" :class="[field, 'pl-9']" aria-label="Sort">
          <option value="newest">Newest first</option>
          <option value="oldest">Oldest first</option>
          <option value="name">Name</option>
          <option value="email">Email</option>
        </select>
      </label>
    </div>

    <p v-if="notice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <div v-if="visible.length" class="mt-4 overflow-x-auto rounded-panel border border-line bg-surface">
      <table class="km-table">
        <thead>
          <tr>
            <th>Email</th>
            <th>Display name</th>
            <th>Role</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="person in visible" :key="person.id">
            <td>
              <span class="flex items-center gap-2 font-medium">
                <User class="size-4 shrink-0 text-muted" />
                <span class="truncate">{{ person.email }}</span>
              </span>
            </td>
            <td class="text-muted">{{ person.displayName }}</td>
            <td>
              <span v-if="person.roles.length" class="flex flex-wrap gap-1">
                <span v-for="name in person.roles" :key="name" class="km-pill" :class="roleTone(name)">
                  <span class="size-1.5 rounded-full bg-current" />
                  {{ roleLabel(name) }}
                </span>
              </span>
              <span v-else class="text-muted">No role</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else class="mt-6 text-sm text-muted">
      {{ query.trim() || role !== "all" ? "Nothing matches that search." : "No account is on this desk." }}
    </p>
    <ListPager v-model="page" :pages="pages" :total="matched.length" />
  </section>
</template>
