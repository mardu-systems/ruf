<script lang="ts">
  import { t, text, message } from "../i18n";
  import type { RegistrationState } from "../generated/AppInfo";
  import type { SettingsSection } from "../api";
  import Icon from "./Icon.svelte";
  let {
    runtime,
    registration,
    hasAccount = false,
    accountEnabled = true,
    callInProgress = false,
    busy = false,
    audioUnknown,
    onsetup,
    onretry,
    onexplore,
  }: {
    runtime: "desktop" | "browser" | "unlinked" | "unavailable" | "loading";
    registration: RegistrationState;
    hasAccount?: boolean;
    accountEnabled?: boolean;
    callInProgress?: boolean;
    busy?: boolean;
    audioUnknown: boolean;
    onsetup: (section: SettingsSection) => void;
    onretry: () => void;
    onexplore: () => void;
  } = $props();
  type StartState =
    | Exclude<typeof runtime, "desktop">
    | "call"
    | "audio"
    | "disabled"
    | "ready"
    | "registering"
    | "failed"
    | "saved"
    | "connect";
  const state = $derived.by((): StartState => {
    if (runtime !== "desktop") return runtime;
    if (callInProgress) return "call";
    if (audioUnknown) return "audio";
    if (!accountEnabled) return "disabled";
    if (registration === "registered") return "ready";
    if (registration === "registering") return "registering";
    if (registration === "failed") return "failed";
    return hasAccount ? "saved" : "connect";
  });
  const description = $derived(
    state === "call" || state === "loading"
      ? null
      : message(`phone.start.${state}_hint`),
  );
</script>

<section
  class="getting-started"
  aria-labelledby="start-title"
  aria-busy={state === "loading"}
>
  <div class="start-symbol"><Icon name="phone" size={22} /></div>
  <h1 id="start-title" aria-live="polite">
    {$t(`phone.start.${state}_title`)}
  </h1>
  {#if description}<p class="start-description">{$text(description)}</p>{/if}
  {#if state === "connect"}
    <p class="start-requirements">
      <strong>{$t("phone.start.requirements_title")}</strong>
      {$t("phone.start.requirements")}
    </p>
  {/if}
  {#if state !== "loading"}<div class="start-actions">
      {#if state === "unavailable"}
        <button class="primary-button" disabled={busy} onclick={onretry}
          >{$t("app.reload")}</button
        >
      {:else if state === "audio" || state === "unlinked"}
        <button
          class="primary-button"
          disabled={busy}
          onclick={() => onsetup("diagnostics")}
          >{$t("phone.start.diagnostics")}</button
        >
      {:else if state === "call" || state === "ready" || state === "browser"}
        <button class="primary-button" disabled={busy} onclick={onexplore}
          >{state === "call"
            ? $t("phone.start.return_call")
            : state === "ready"
              ? $t("phone.start.dial")
              : $t("phone.start.explore")}</button
        >
      {:else}
        <button
          class="primary-button"
          disabled={busy}
          onclick={() => onsetup("accounts")}
          >{state === "failed"
            ? $t("phone.start.check_connection")
            : state === "connect"
              ? $t("phone.start.setup")
              : $t("phone.start.account_settings")}</button
        >
      {/if}
      {#if state !== "call" && state !== "ready" && state !== "browser"}
        <button class="text-button" disabled={busy} onclick={onexplore}
          >{$t("phone.start.keypad")}</button
        >
      {/if}
    </div>{/if}
</section>
