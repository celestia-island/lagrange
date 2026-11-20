import { createApp } from "vue";
import { initTheme } from "@celestia-island/hikari";
import "@celestia-island/hikari/styles";
import App from "./App.vue";

initTheme();
createApp(App).mount("#app");
