import { mount } from "svelte";
import { isTauri } from "@tauri-apps/api/core";
import "./app.css";
import { initializeLocalization, locale, t } from "./lib/i18n";

const target = document.getElementById("app");
if (!target) throw new Error("Application mount element is missing.");

const settingsWindow =
  new URLSearchParams(window.location.search).get("window") === "settings";
document.documentElement.classList.toggle("settings-document", settingsWindow);
document.documentElement.classList.toggle("native-document", isTauri());
await initializeLocalization();
locale.subscribe((language) => {
  document.documentElement.lang = language;
});
try {
  const { default: WindowComponent } = settingsWindow
    ? await import("./Settings.svelte")
    : await import("./App.svelte");
  mount(WindowComponent, { target });
} catch {
  // This recovery path must work even when the Svelte window chunk cannot load.
  const shell = document.createElement("div");
  shell.className = "softphone";
  const content = document.createElement("main");
  content.className = "getting-started";
  content.setAttribute("aria-labelledby", "startup-error-title");
  const title = document.createElement("h1");
  title.id = "startup-error-title";
  const description = document.createElement("p");
  description.className = "start-description";
  description.setAttribute("role", "alert");
  const actions = document.createElement("div");
  actions.className = "start-actions";
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "primary-button";
  const unsubscribe = t.subscribe((translate) => {
    title.textContent = translate("app.window_load_failed");
    description.textContent = translate("app.window_load_failed_hint");
    retry.textContent = translate("app.reload_window");
  });
  window.addEventListener("pagehide", () => unsubscribe(), { once: true });
  retry.addEventListener("click", () => window.location.reload());
  actions.append(retry);
  content.append(title, description, actions);
  shell.append(content);
  target.replaceChildren(shell);
  retry.focus();
}
