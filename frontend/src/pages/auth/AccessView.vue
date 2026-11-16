<script setup lang="ts">
import dayjs from "dayjs";
import { computed, onMounted, ref } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import type { DeskSession, PersonAccount } from "../../services/auth.service";
import { fetchAudit, type AuditEntry } from "../../services/audit.service";
import { fetchProviderStatus, syncProvider, type ProviderStatus } from "../../services/provider.service";
import { errorMessage } from "../../services/http";
import { safeReturnPath } from "../../services/mailbox.service";
import { useAuthStore } from "../../stores/auth";
import { useMailboxStore } from "../../stores/mailboxes";

const auth = useAuthStore();
const boxes = useMailboxStore();
const route = useRoute();
const router = useRouter();

const email = ref("");
const password = ref("");
const formError = ref("");
const busy = ref(false);

const name = ref("");
const currentPassword = ref("");
const nextPassword = ref("");
const sessions = ref<DeskSession[]>([]);
const people = ref<PersonAccount[]>([]);
const mailboxAddress = ref("");
const mailboxName = ref("");
const grantEmail = ref("");
const grantMailboxId = ref("");
const grantSend = ref(false);
const grantManage = ref(false);
const personEmail = ref("");
const personName = ref("");
const personPassword = ref("");
const personRole = ref<"member" | "administrator">("member");
const notice = ref("");
const provider = ref<ProviderStatus | null>(null);
const audit = ref<AuditEntry[]>([]);

const manageable = computed(() => boxes.items.filter((mailbox) => mailbox.canManage));
const field =
  "h-10 w-full rounded-control border border-line bg-surface px-3 text-sm text-ink outline-none focus:border-lagoon";
const primary =
  "inline-flex h-10 items-center rounded-full bg-lagoon px-4 text-sm font-semibold text-lagoon-ink transition-colors duration-200 ease-out hover:bg-brass hover:text-brass-ink disabled:opacity-60";
const quiet = "inline-flex h-9 items-center rounded-control px-3 text-sm hover:bg-canvas";

onMounted(async () => {
  if (auth.status !== "signed_in") {
    return;
  }
  name.value = auth.user?.displayName ?? "";
  await loadAccount();
});

async function loadAccount() {
  notice.value = "";
  try {
    await boxes.ensure();
    sessions.value = await auth.sessions();
    provider.value = auth.canManageMailboxes ? await fetchProviderStatus() : null;
    audit.value = auth.canReadAudit ? await fetchAudit() : [];
    people.value = auth.canManageUsers ? await auth.people() : [];
    if (!grantMailboxId.value) {
      grantMailboxId.value = manageable.value[0]?.id ?? "";
    }
  } catch (error) {
    notice.value = errorMessage(error);
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

async function submitName() {
  notice.value = "";
  try {
    await auth.saveName(name.value);
    notice.value = "Name updated.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function submitPassword() {
  notice.value = "";
  try {
    notice.value = await auth.savePassword(currentPassword.value, nextPassword.value);
    currentPassword.value = "";
    nextPassword.value = "";
    sessions.value = await auth.sessions();
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

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

async function checkProvider() {
  notice.value = "";
  try {
    provider.value = await syncProvider();
    await boxes.reload();
    notice.value = provider.value.configured
      ? `${provider.value.sendingDomain} is ${provider.value.domainStatus}. Receiving is ${provider.value.receiving}.`
      : "Resend is not configured on the server.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function submitMailbox() {
  notice.value = "";
  try {
    await boxes.register(mailboxAddress.value, mailboxName.value);
    mailboxAddress.value = "";
    mailboxName.value = "";
    grantMailboxId.value = manageable.value[0]?.id ?? "";
    notice.value = "Mailbox registered. Receiving stays off until the provider route is confirmed.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
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

async function submitGrant() {
  notice.value = "";
  try {
    await boxes.grant(grantMailboxId.value, {
      email: grantEmail.value,
      canRead: true,
      canSend: grantSend.value,
      canManage: grantManage.value,
    });
    grantEmail.value = "";
    grantSend.value = false;
    grantManage.value = false;
    notice.value = "Access granted.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function submitPerson() {
  notice.value = "";
  try {
    await auth.addPerson({
      email: personEmail.value,
      displayName: personName.value,
      password: personPassword.value,
      role: personRole.value,
    });
    personEmail.value = "";
    personName.value = "";
    personPassword.value = "";
    people.value = await auth.people();
    notice.value = "Account created.";
  } catch (error) {
    notice.value = errorMessage(error);
  }
}

async function leave() {
  await auth.logout();
}

function when(iso: string) {
  return dayjs(iso).format("D MMM YYYY, HH:mm");
}
</script>

<template>
  <section v-if="auth.status !== 'signed_in'" class="max-w-[28rem]">
    <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Sign in</h1>
    <p class="mt-3 text-base leading-relaxed text-muted">
      Use the account issued for this desk. Password reset by email is not available yet.
    </p>
    <p v-if="auth.unreachable" class="mt-4 text-sm text-danger" role="alert">
      Mail service is not reachable.
    </p>
    <form class="mt-8 space-y-4" @submit.prevent="submitLogin">
      <label class="block text-sm font-medium">
        Email
        <input v-model="email" type="email" autocomplete="username" required :class="['mt-1.5', field]" />
      </label>
      <label class="block text-sm font-medium">
        Password
        <input
          v-model="password"
          type="password"
          autocomplete="current-password"
          required
          :class="['mt-1.5', field]"
        />
      </label>
      <p v-if="formError" class="text-sm text-danger" role="alert">{{ formError }}</p>
      <button type="submit" :class="primary" :disabled="busy">
        {{ busy ? "Signing in…" : "Sign in" }}
      </button>
    </form>
  </section>

  <section v-else>
    <div class="flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold leading-tight tracking-[-0.03em]">
          {{ auth.user?.displayName }}
        </h1>
        <p class="mt-2 text-sm text-muted">{{ auth.user?.email }} · {{ auth.user?.roles.join(", ") }}</p>
      </div>
      <div class="flex gap-2">
        <RouterLink to="/" :class="primary">Open the desk</RouterLink>
        <button type="button" class="h-10 rounded-full border border-line px-4 text-sm" @click="leave">
          Sign out
        </button>
      </div>
    </div>

    <p v-if="notice" class="mt-6 rounded-control bg-surface px-3 py-2 text-sm leading-relaxed" role="status">
      {{ notice }}
    </p>

    <form class="mt-10 max-w-md space-y-3" @submit.prevent="submitName">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Name</h2>
      <input v-model="name" required maxlength="80" :class="field" aria-label="Display name" />
      <button type="submit" :class="primary">Save name</button>
    </form>

    <form class="mt-10 max-w-md space-y-3" @submit.prevent="submitPassword">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Password</h2>
      <p class="text-sm leading-relaxed text-muted">
        12 to 128 characters. Other signed-in sessions on this account end when the password changes.
        Two-factor sign-in is not available.
      </p>
      <input
        v-model="currentPassword"
        type="password"
        autocomplete="current-password"
        required
        placeholder="Current password"
        :class="field"
        aria-label="Current password"
      />
      <input
        v-model="nextPassword"
        type="password"
        autocomplete="new-password"
        required
        minlength="12"
        maxlength="128"
        placeholder="New password"
        :class="field"
        aria-label="New password"
      />
      <button type="submit" :class="primary">Change password</button>
    </form>

    <section class="mt-10">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Sessions</h2>
      <ul class="mt-3 divide-y divide-line rounded-control border border-line bg-surface">
        <li v-for="session in sessions" :key="session.id" class="flex items-center gap-3 px-3 py-3">
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm">{{ session.userAgent || "Unknown browser" }}</p>
            <p class="text-xs text-muted">
              Started {{ when(session.createdAt) }}
              <span v-if="session.current"> · This session</span>
            </p>
          </div>
          <button type="button" :class="quiet" @click="endSession(session)">End</button>
        </li>
      </ul>
    </section>

    <section v-if="auth.canManageMailboxes && provider" class="mt-10 max-w-xl">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Email provider</h2>
      <p class="mt-2 text-sm leading-relaxed text-muted">
        Sending goes through Resend for @{{ provider.sendingDomain }}. The compose screen sends through this
        provider. Checking with Resend does not create a receiving address. A mailbox stays unverified until
        Resend lists that inbox or a message arrives for it.
      </p>
      <ul class="mt-3 space-y-1 text-sm">
        <li>{{ provider.configured ? "The API key is set on the server." : "Resend is not configured on the server." }}</li>
        <li>
          {{
            provider.webhookConfigured
              ? "Webhook signing is set."
              : "Webhook signing is not set, so inbound events are refused."
          }}
        </li>
        <li>
          Webhook queue: {{ provider.queueReceived }} waiting, {{ provider.queueProcessing }} processing,
          {{ provider.queueFailed }} failed.
        </li>
        <li v-if="provider.queueOldestSeconds !== null">
          The oldest waiting event is {{ provider.queueOldestSeconds }} seconds old.
        </li>
        <li>
          {{
            provider.scanner === "clamav"
              ? "ClamAV is configured."
              : "Malware scanning is not configured. Attachments stay marked not scanned."
          }}
        </li>
        <li v-if="provider.domainStatus !== 'not_checked'">
          {{ provider.sendingDomain }} is {{ provider.domainStatus }}. Receiving is {{ provider.receiving }}.
        </li>
      </ul>
      <button type="button" class="mt-4" :class="primary" @click="checkProvider">Check with Resend</button>
    </section>

    <section v-if="auth.canReadAudit" class="mt-10 max-w-xl">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Audit log</h2>
      <p v-if="audit.length === 0" class="mt-2 text-sm text-muted">No audit records yet.</p>
      <ul v-else class="mt-3 divide-y divide-line rounded-control border border-line bg-surface">
        <li v-for="entry in audit" :key="entry.id" class="px-3 py-3">
          <p class="text-sm">{{ entry.action }}</p>
          <p class="text-xs text-muted">
            {{ entry.actorEmail || "Unknown person" }} · {{ when(entry.createdAt) }}
          </p>
        </li>
      </ul>
    </section>

    <section class="mt-10">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">Mailboxes</h2>
      <p class="mt-2 max-w-[62ch] text-sm leading-relaxed text-muted">
        Enabling a mailbox does not start receiving. A mailbox stays unverified until its provider route is confirmed.
      </p>
      <ul v-if="boxes.items.length" class="mt-4 divide-y divide-line rounded-control border border-line bg-surface">
        <li v-for="mailbox in boxes.items" :key="mailbox.id" class="flex flex-wrap items-center gap-3 px-3 py-3">
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium">{{ mailbox.address }}</p>
            <p class="text-xs text-muted">
              {{ mailbox.displayName }}
              · {{ mailbox.isEnabled ? "Enabled" : "Disabled" }}
              · {{ mailbox.isReceivingVerified ? "Receiving verified" : "Receiving not verified" }}
              <span v-if="mailbox.isDefault"> · Default</span>
            </p>
          </div>
          <button v-if="mailbox.canRead" type="button" :class="quiet" @click="useMailbox(mailbox.id)">Open</button>
          <button
            v-if="mailbox.canManage"
            type="button"
            :class="quiet"
            @click="toggleEnabled(mailbox.id, !mailbox.isEnabled)"
          >
            {{ mailbox.isEnabled ? "Disable" : "Enable" }}
          </button>
        </li>
      </ul>
      <p v-else class="mt-3 text-sm text-muted">No mailbox is available to this account.</p>

      <form v-if="auth.canManageMailboxes" class="mt-6 max-w-md space-y-3" @submit.prevent="submitMailbox">
        <h3 class="text-sm font-semibold">Register a mailbox</h3>
        <input v-model="mailboxName" required maxlength="80" placeholder="Display name" :class="field" aria-label="Mailbox display name" />
        <input v-model="mailboxAddress" type="email" required placeholder="name@kamfany.com" :class="field" aria-label="Mailbox address" />
        <button type="submit" :class="primary">Register mailbox</button>
      </form>

      <form v-if="manageable.length" class="mt-6 max-w-md space-y-3" @submit.prevent="submitGrant">
        <h3 class="text-sm font-semibold">Grant access</h3>
        <p class="text-sm text-muted">The person needs an account first. Send and manage include reading.</p>
        <select v-model="grantMailboxId" :class="field" aria-label="Mailbox to share">
          <option v-for="mailbox in manageable" :key="mailbox.id" :value="mailbox.id">
            {{ mailbox.address }}
          </option>
        </select>
        <input v-model="grantEmail" type="email" required placeholder="person@kamfany.com" :class="field" aria-label="Person email" />
        <label class="flex items-center gap-2 text-sm">
          <input v-model="grantSend" type="checkbox" class="size-4 accent-brass" />
          Can send
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input v-model="grantManage" type="checkbox" class="size-4 accent-brass" />
          Can manage
        </label>
        <button type="submit" :class="primary">Grant access</button>
      </form>
    </section>

    <section v-if="auth.canManageUsers" class="mt-10">
      <h2 class="text-lg font-semibold tracking-[-0.02em]">People</h2>
      <ul class="mt-3 divide-y divide-line rounded-control border border-line bg-surface">
        <li v-for="person in people" :key="person.id" class="px-3 py-3">
          <p class="text-sm font-medium">{{ person.displayName }}</p>
          <p class="text-xs text-muted">{{ person.email }} · {{ person.roles.join(", ") }}</p>
        </li>
      </ul>
      <form class="mt-6 max-w-md space-y-3" @submit.prevent="submitPerson">
        <h3 class="text-sm font-semibold">Create an account</h3>
        <input v-model="personName" required maxlength="80" placeholder="Display name" :class="field" aria-label="New person name" />
        <input v-model="personEmail" type="email" required placeholder="person@kamfany.com" :class="field" aria-label="New person email" />
        <input
          v-model="personPassword"
          type="password"
          required
          minlength="12"
          maxlength="128"
          autocomplete="new-password"
          placeholder="Temporary password"
          :class="field"
          aria-label="Temporary password"
        />
        <select v-model="personRole" :class="field" aria-label="Role">
          <option value="member">Member</option>
          <option value="administrator">Administrator</option>
        </select>
        <button type="submit" :class="primary">Create account</button>
      </form>
    </section>
  </section>
</template>
