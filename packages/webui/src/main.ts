import { createSSRApp } from "vue";
import { initTheme } from "@celestia-island/hikari";
import "@celestia-island/hikari/styles";
import App from "./App";

const app = createSSRApp(App);
initTheme();
app.mount("#app");
