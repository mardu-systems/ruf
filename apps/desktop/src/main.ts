import { mount } from "svelte";
import { isTauri } from "@tauri-apps/api/core";
import "./app.css";
import { initializeLocalization, locale } from "./lib/i18n";

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
const { default: WindowComponent } = settingsWindow
  ? await import("./Settings.svelte")
  : await import("./App.svelte");
mount(WindowComponent, { target });
