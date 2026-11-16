<script setup lang="ts">
import dayjs from "dayjs";
import { computed, onMounted, ref } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import { requestPasswordReset, type DeskSession } from "../../services/auth.service";
import { errorMessage } from "../../services/http";
import { safeReturnPath } from "../../services/mailbox.service";
import { useAuthStore } from "../../stores/auth";
import { useMailboxStore } from "../../stores/mailboxes";
import { field, panel } from "./account-form";

const auth = useAuthStore();
const boxes = useMailboxStore();
const route = useRoute();
const router = useRouter();

const email = ref("");
const password = ref("");
const formError = ref("");
const busy = ref(false);
const forgot = ref(false);
const resetNotice = ref("");
const name = ref("");
const notice = ref("");
const loadNotice = ref("");
const sessions = ref<DeskSession[]>([]);
const sessionsLoaded = ref(false);

const ACCESS = [
  ["mail.read", "Read messages in granted mailboxes"],
  ["mail.send", "Send messages from granted mailboxes"],
  ["mail.organize", "Move, label, archive, and delete messages"],
  ["mailbox.manage", "Create and configure mailboxes"],
  ["contacts.manage", "Manage personal contacts"],
  ["users.manage", "Manage user accounts and roles"],
  ["audit.read", "Read the audit log"],
] as const;

const currentSession = computed(() => sessions.value.find((session) => session.current) ?? null);
const otherSessions = computed(() => Math.max(0, sessions.value.length - (currentSession.value ? 1 : 0)));
const mailboxPreview = computed(() => boxes.items.slice(0, 6));
const access = computed(() => {
  const keys = auth.user?.permissions ?? [];
  const known = new Set<string>(ACCESS.map(([key]) => key));
  const listed = ACCESS.filter(([key]) => keys.includes(key)).map(([, label]) => label);
  return [...listed, ...keys.filter((key) => !known.has(key))];
});
const gateField =
  "h-11 w-full rounded-control border border-line bg-canvas px-3 text-sm text-ink outline-none focus:border-lagoon";
const gatePrimary =
  "mt-1 inline-flex h-11 w-full items-center justify-center rounded-full bg-lagoon px-4 text-sm font-semibold text-lagoon-ink transition-colors duration-200 ease-out hover:bg-brass hover:text-brass-ink disabled:opacity-60";

onMounted(async () => {
  if (auth.status !== "signed_in") {
    return;
  }
  name.value = auth.user?.displayName ?? "";
  try {
    await boxes.ensure();
  } catch (error) {
    loadNotice.value = errorMessage(error);
  }
  try {
    sessions.value = await auth.sessions();
  } catch (error) {
    loadNotice.value = errorMessage(error);
  } finally {
    sessionsLoaded.value = true;
  }
});

function roleLabel(role: string): string {
  if (role === "owner") {
    return "Owner";
  }
  if (role === "administrator") {
    return "Administrator";
  }
  if (role === "member") {
    return "Member";
  }
  return role;
}

function roleTone(role: string): string {
  if (role === "owner") {
    return "mint";
  }
  if (role === "administrator") {
    return "sand";
  }
  return "quiet";
}

async function submitReset() {
  formError.value = "";
  resetNotice.value = "";
  busy.value = true;
  try {
    resetNotice.value = await requestPasswordReset(email.value);
  } catch (error) {
    formError.value = errorMessage(error);
  } finally {
    busy.value = false;
  }
}

async function submitLogin() {
  formError.value = "";
  busy.value = true;
  try {
    await auth.login(email.value, password.value);
    await boxes.ensure();
    await router.push(safeReturnPath(route.query.next));
  } catch (error) {
    formError.value = errorMessage(error);
  } finally {
    busy.value = false;
  }
}

function toggleForgot() {
  forgot.value = !forgot.value;
  formError.value = "";
  resetNotice.value = "";
}

async function submitName() {
  notice.value = "";
  try {
    await auth.saveName(name.value);
    notice.value = "Name updated.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function leave() {
  await auth.logout();
}
</script>

<template>
  <section v-if="auth.status !== 'signed_in'">
    <h1 class="text-balance text-[1.75rem] font-semibold leading-tight tracking-[-0.03em]">
      {{ forgot ? "Reset password" : "Sign in" }}
    </h1>
    <p class="mt-2 text-sm leading-relaxed text-muted">
      {{
        forgot
          ? "Enter the account email. If mail delivery is connected, a reset link is sent there."
          : "Use the account issued for this desk."
      }}
    </p>
    <p v-if="auth.unreachable" class="mt-4 text-sm text-danger" role="alert">
      Mail service is not reachable.
    </p>
    <form class="mt-7 space-y-4" @submit.prevent="forgot ? submitReset() : submitLogin()">
      <label class="block text-sm font-medium">
        Email
        <input v-model="email" type="email" autocomplete="username" required :class="['mt-1.5', gateField]" />
      </label>
      <label v-if="!forgot" class="block text-sm font-medium">
        Password
        <input
          v-model="password"
          type="password"
          autocomplete="current-password"
          required
          :class="['mt-1.5', gateField]"
        />
      </label>
      <p v-if="formError" class="text-sm text-danger" role="alert">{{ formError }}</p>
      <p v-if="resetNotice" class="text-sm leading-relaxed text-muted" role="status">{{ resetNotice }}</p>
      <button type="submit" :class="gatePrimary" :disabled="busy">
        {{ busy ? (forgot ? "Sending…" : "Signing in…") : forgot ? "Send reset link" : "Sign in" }}
      </button>
    </form>
    <button
      type="button"
      class="mt-5 block w-full text-center text-sm text-muted hover:text-ink"
      @click="toggleForgot"
    >
      {{ forgot ? "Back to sign in" : "Forgot password" }}
    </button>
  </section>

  <section v-else>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Account settings</h1>
        <p class="mt-2 text-sm text-muted">{{ auth.user?.email }}</p>
        <p v-if="auth.user?.roles.length" class="mt-3 flex flex-wrap gap-1">
          <span
            v-for="role in auth.user.roles"
            :key="role"
            class="km-pill"
            :class="roleTone(role)"
          >
            <span class="size-1.5 rounded-full bg-current" />
            {{ roleLabel(role) }}
          </span>
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <RouterLink to="/" class="km-action">Open the desk</RouterLink>
        <button type="button" class="km-quiet" @click="leave">Sign out</button>
      </div>
    </div>

    <p v-if="loadNotice" class="mt-4 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ loadNotice }}
    </p>

    <div class="mt-4 grid gap-3 min-[900px]:grid-cols-2">
      <form :class="['space-y-3', panel]" @submit.prevent="submitName">
        <h2 class="text-lg font-semibold tracking-[-0.02em]">Display name</h2>
        <p class="text-sm leading-relaxed text-muted">This is the name shown for this account.</p>
        <input v-model="name" required maxlength="80" :class="[field, 'bg-canvas']" aria-label="Display name" />
        <p v-if="notice" class="text-sm leading-relaxed" role="status">{{ notice }}</p>
        <button type="submit" class="km-action">Save name</button>
      </form>

      <article :class="panel">
        <div class="flex items-start justify-between gap-3">
          <h2 class="text-lg font-semibold tracking-[-0.02em]">This session</h2>
          <RouterLink to="/account/sessions" class="text-sm font-semibold text-ink hover:underline">All sessions</RouterLink>
        </div>
        <p v-if="!sessionsLoaded" class="mt-3 text-sm text-muted">Checking signed-in browsers.</p>
        <template v-else-if="currentSession">
          <p class="mt-3 truncate text-sm">{{ currentSession.userAgent || "Unknown browser" }}</p>
          <p class="mt-1 text-sm text-muted">Started {{ dayjs(currentSession.createdAt).format("D MMM YYYY, HH:mm") }}</p>
          <p class="mt-3 text-sm text-muted">
            {{
              otherSessions === 0
                ? "No other browser is signed in."
                : otherSessions === 1
                  ? "1 other browser is signed in."
                  : `${otherSessions} other browsers are signed in.`
            }}
          </p>
        </template>
        <p v-else class="mt-3 text-sm text-muted">This browser is not listed among the open sessions.</p>
      </article>
    </div>

    <div class="mt-3 grid gap-3 min-[900px]:grid-cols-2">
      <article :class="panel">
        <div class="flex items-start justify-between gap-3">
          <h2 class="text-lg font-semibold tracking-[-0.02em]">Mailboxes</h2>
          <RouterLink to="/account/mailboxes" class="text-sm font-semibold text-ink hover:underline">View mailboxes</RouterLink>
        </div>
        <ul v-if="mailboxPreview.length" class="mt-3 divide-y divide-line">
          <li v-for="mailbox in mailboxPreview" :key="mailbox.id" class="flex items-center justify-between gap-3 py-2">
            <span class="min-w-0">
              <span class="block truncate text-sm font-medium">{{ mailbox.address }}</span>
              <span class="block truncate text-xs text-muted">{{ mailbox.displayName }}</span>
            </span>
            <span
              class="km-pill shrink-0"
              :class="mailbox.isEnabled ? 'mint' : 'rose'"
            >
              <span class="size-1.5 rounded-full bg-current" />
              {{ mailbox.isEnabled ? "Enabled" : "Disabled" }}
            </span>
          </li>
        </ul>
        <p v-else class="mt-3 text-sm text-muted">No mailbox is available to this account.</p>
        <p v-if="boxes.items.length > mailboxPreview.length" class="mt-2 text-xs text-muted">
          {{ boxes.items.length - mailboxPreview.length }} more on the mailboxes page.
        </p>
      </article>

      <article :class="panel">
        <h2 class="text-lg font-semibold tracking-[-0.02em]">What you can do</h2>
        <ul v-if="access.length" class="mt-3 space-y-2">
          <li v-for="item in access" :key="item" class="flex gap-2 text-sm leading-relaxed">
            <span class="mt-2 size-1.5 shrink-0 rounded-full bg-current text-[var(--km-mail-mint)]" />
            <span>{{ item }}</span>
          </li>
        </ul>
        <p v-else class="mt-3 text-sm text-muted">This account has no listed permissions.</p>
      </article>
    </div>
  </section>
</template>
