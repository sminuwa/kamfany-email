<script setup lang="ts">
import { RouterView } from "vue-router";
import AuthGate from "../components/layout/AuthGate.vue";
</script>

<template>
  <AuthGate>
    <RouterView />
  </AuthGate>
</template>
