import { createPinia } from "pinia";
import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { useBrandStore } from "./stores/brand";
import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/public-sans/700.css";
import "./styles/tokens.css";
import "./styles/base.css";

const pinia = createPinia();
const app = createApp(App);
app.use(pinia);
app.use(router);
await useBrandStore(pinia).load();
app.mount("#app");
