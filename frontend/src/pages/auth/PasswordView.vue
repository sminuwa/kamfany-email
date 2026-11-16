<script setup lang="ts">
import { ref } from "vue";
import { errorMessage } from "../../services/http";
import { useAuthStore } from "../../stores/auth";
import { field, panel } from "./account-form";

const auth = useAuthStore();
const currentPassword = ref("");
const nextPassword = ref("");
const confirmPassword = ref("");
const notice = ref("");
const formError = ref("");

async function submit() {
  notice.value = "";
  formError.value = "";
  if (nextPassword.value !== confirmPassword.value) {
    formError.value = "Enter the same password twice.";
    return;
  }
  try {
    notice.value = await auth.savePassword(currentPassword.value, nextPassword.value);
    currentPassword.value = "";
    nextPassword.value = "";
    confirmPassword.value = "";
  } catch (error) {
    formError.value = errorMessage(error);
  }
}
</script>

<template>
  <section class="mx-auto flex min-h-[calc(100dvh-3.5rem-3rem)] w-full max-w-md flex-col justify-center">
    <h1 class="text-center text-[2rem] font-semibold leading-tight tracking-[-0.03em]">Change password</h1>
    <p class="mt-3 text-center text-sm leading-relaxed text-muted">
      12 to 128 characters. Other signed-in sessions on this account end when the password changes.
      Two-factor sign-in is not available.
    </p>
    <form :class="['mt-8 space-y-3', panel]" @submit.prevent="submit">
      <input
        v-model="currentPassword"
        type="password"
        autocomplete="current-password"
        required
        placeholder="Current password"
        :class="[field, 'bg-canvas']"
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
        :class="[field, 'bg-canvas']"
        aria-label="New password"
      />
      <input
        v-model="confirmPassword"
        type="password"
        autocomplete="new-password"
        required
        minlength="12"
        maxlength="128"
        placeholder="Confirm new password"
        :class="[field, 'bg-canvas']"
        aria-label="Confirm new password"
      />
      <p v-if="formError" class="text-sm text-danger" role="alert">{{ formError }}</p>
      <p v-if="notice" class="text-sm leading-relaxed text-muted" role="status">{{ notice }}</p>
      <button type="submit" :class="['km-action', 'w-full']">Change password</button>
    </form>
  </section>
</template>
