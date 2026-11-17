<script setup lang="ts">
import { ref } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { resetPassword } from "../../services/auth.service";
import { errorMessage } from "../../services/http";

const route = useRoute();
const password = ref("");
const confirm = ref("");
const notice = ref("");
const formError = ref("");
const busy = ref(false);

const token = typeof route.query.token === "string" ? route.query.token : "";
const field =
  "h-11 w-full rounded-control border border-line bg-canvas px-3 text-sm text-ink outline-none focus:border-lagoon";
const primary =
  "mt-1 inline-flex h-11 w-full items-center justify-center rounded-full bg-lagoon px-4 text-sm font-semibold text-lagoon-ink transition-colors duration-200 ease-out hover:bg-brass hover:text-brass-ink disabled:opacity-60";

async function submit() {
  formError.value = "";
  notice.value = "";
  if (password.value !== confirm.value) {
    formError.value = "Enter the same password twice.";
    return;
  }
  busy.value = true;
  try {
    notice.value = await resetPassword(token, password.value);
    password.value = "";
    confirm.value = "";
  } catch (error) {
    formError.value = errorMessage(error);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section>
    <h1 class="text-balance text-[1.75rem] font-semibold leading-tight tracking-[-0.03em]">Choose a new password</h1>
    <p class="mt-2 text-sm leading-relaxed text-muted">
      The link works once, for 30 minutes. Signing in afterwards uses the new password.
    </p>
    <p v-if="!token" class="mt-4 text-sm text-danger" role="alert">This reset link is not valid.</p>
    <form v-else class="mt-8 space-y-4" @submit.prevent="submit">
      <label class="block text-sm font-medium">
        New password
        <input
          v-model="password"
          type="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          required
          :class="['mt-1.5', field]"
        />
      </label>
      <label class="block text-sm font-medium">
        Repeat the password
        <input
          v-model="confirm"
          type="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          required
          :class="['mt-1.5', field]"
        />
      </label>
      <p v-if="formError" class="text-sm text-danger" role="alert">{{ formError }}</p>
      <p v-if="notice" class="text-sm leading-relaxed text-muted" role="status">{{ notice }}</p>
      <button type="submit" :class="primary" :disabled="busy">
        {{ busy ? "Saving…" : "Update password" }}
      </button>
    </form>
    <RouterLink to="/account" class="mt-5 block text-center text-sm text-muted hover:text-ink">Back to sign in</RouterLink>
  </section>
</template>
