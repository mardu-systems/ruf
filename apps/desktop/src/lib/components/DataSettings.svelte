<script lang="ts">
  import { t, text, locale, message, type MessageValue } from "../i18n";
  import { tick } from "svelte";
  import { getCountries } from "libphonenumber-js/max";
  import type { AppPreferences } from "../generated/AppInfo";
  import { clearHistory } from "../history-api";
  import { errorMessage } from "../api";
  import { defaultNumberRegion } from "../contacts";
  let {
    value = $bindable(),
    dirty,
    disabled,
    clearDisabled,
    callActive,
    recordingDirty,
    savedRetentionDays,
    unknownValues,
    confirmRetention = $bindable(false),
    onsave,
    ondiscard,
  }: {
    value: AppPreferences;
    dirty: boolean;
    disabled: boolean;
    clearDisabled: boolean;
    callActive: boolean;
    recordingDirty: boolean;
    savedRetentionDays: number | null;
    unknownValues: MessageValue | null;
    confirmRetention: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
  let confirmClear = $state(false),
    clearing = $state(false),
    feedback = $state<MessageValue>("");
  let clearError = $state<MessageValue | null>(null);
  let cancelButton = $state<HTMLButtonElement>();
  let clearButton = $state<HTMLButtonElement>();
  const retentionShortened = $derived(
    value.history.retentionDays !== null &&
      (savedRetentionDays === null ||
        value.history.retentionDays < savedRetentionDays),
  );
  const regions = $derived.by(() => {
    const language = $locale;
    const names = new Intl.DisplayNames(language, { type: "region" });
    return getCountries()
      .map((code) => ({ code, name: names.of(code) ?? code }))
      .sort((a, b) => a.name.localeCompare(b.name, language));
  });
  async function showClearConfirmation(show: boolean) {
    confirmClear = show;
    clearError = null;
    feedback = "";
    await tick();
    (show ? cancelButton : clearButton)?.focus();
  }
  async function clear() {
    if (!confirmClear || clearing || clearDisabled) return;
    clearing = true;
    clearError = null;
    try {
      await clearHistory();
      feedback = message("settings.history_cleared");
      confirmClear = false;
    } catch (e) {
      clearError = errorMessage(e);
    } finally {
      clearing = false;
    }
    if (!confirmClear) {
      await tick();
      clearButton?.focus();
    }
  }
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.contacts_history")}</h1>
    <p>{$t("settings.data_description")}</p>
  </div>
</div>
{#if unknownValues}
  <p class="settings-footnote">{$text(unknownValues)}</p>
{:else}<h2 class="settings-group" id="contacts-display" tabindex="-1">
    {$t("settings.contact_display")}
  </h2>
  <fieldset disabled={disabled || clearing}>
    <div class="setting-row">
      <div class="setting-description">
        <label for="favorites-first">{$t("settings.favorites_first")}</label>
      </div>
      <input
        id="favorites-first"
        type="checkbox"
        bind:checked={value.contacts.favoritesFirst}
      />
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="show-organization">{$t("settings.show_company")}</label>
      </div>
      <input
        id="show-organization"
        type="checkbox"
        bind:checked={value.contacts.showOrganization}
      />
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="match-national-numbers"
          >{$t("settings.match_national_numbers")}</label
        >
        <p id="match-national-numbers-help">
          {$t("settings.match_national_numbers_help")}
        </p>
      </div>
      <input
        id="match-national-numbers"
        type="checkbox"
        bind:checked={value.contacts.matchGermanNumbers}
        aria-describedby="match-national-numbers-help"
      />
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="number-region">{$t("settings.number_region")}</label>
        <p id="number-region-help">{$t("settings.number_region_help")}</p>
      </div>
      <select
        id="number-region"
        value={value.contacts.numberRegion ?? defaultNumberRegion}
        disabled={!value.contacts.matchGermanNumbers}
        onchange={(event) =>
          (value.contacts.numberRegion = event.currentTarget.value)}
        aria-describedby="number-region-help"
      >
        {#each regions as region (region.code)}
          <option value={region.code}>{region.name}</option>
        {/each}
      </select>
    </div>
    <h2 class="settings-group sound-group">{$t("settings.call_history")}</h2>
    <div class="setting-row">
      <div class="setting-description">
        <label for="history-recording">{$t("settings.record_history")}</label>
        <p id="history-recording-help">
          {$t("settings.record_history_help")}
        </p>
        {#if callActive}<p id="history-recording-lock" role="status">
            {$t(
              recordingDirty
                ? "settings.recording_undo_during_call"
                : "settings.recording_locked_during_call",
            )}
          </p>{/if}
      </div>
      <input
        id="history-recording"
        type="checkbox"
        bind:checked={value.history.recordingEnabled}
        disabled={callActive && !recordingDirty}
        aria-describedby={callActive
          ? "history-recording-help history-recording-lock"
          : "history-recording-help"}
      />
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="history-retention">{$t("settings.retention")}</label>
        <p>
          {$t("settings.retention_help")}
        </p>
      </div>
      <select
        id="history-retention"
        bind:value={value.history.retentionDays}
        onchange={() => (confirmRetention = false)}
        ><option value={null}>{$t("settings.unlimited")}</option><option
          value={30}>{$t("settings.retention_30")}</option
        ><option value={90}>{$t("settings.retention_90")}</option><option
          value={365}>{$t("settings.retention_365")}</option
        ></select
      >
    </div>
    {#if retentionShortened}<div class="setting-row">
        <div class="setting-description">
          <label for="confirm-retention"
            >{$t("settings.confirm_retention")}</label
          >
          <p>
            {$t("settings.confirm_retention_help")}
          </p>
        </div>
        <input
          id="confirm-retention"
          type="checkbox"
          bind:checked={confirmRetention}
        />
      </div>{/if}
    <div class="settings-form-actions settings-save-bar">
      <p>
        {dirty ? $t("settings.unsaved_notice") : ""}
      </p>
      <div class="settings-actions">
        <button
          class="text-button"
          type="button"
          disabled={!dirty}
          onclick={ondiscard}>{$t("settings.discard")}</button
        ><button
          class="primary-button"
          type="button"
          disabled={!dirty || (callActive && recordingDirty)}
          onclick={onsave}>{$t("settings.save_data")}</button
        >
      </div>
    </div>
  </fieldset>{/if}
<h2 class="settings-group sound-group">{$t("settings.clear_history")}</h2>
<p class="settings-footnote">
  {$t("settings.clear_history_help")}
</p>
{#if confirmClear}<div
    class="settings-feedback"
    role="group"
    aria-labelledby="clear-history-question"
  >
    <p id="clear-history-question">{$t("settings.clear_history_question")}</p>
    <div class="settings-actions">
      <button
        type="button"
        class="secondary-button"
        bind:this={cancelButton}
        disabled={clearing}
        onclick={() => void showClearConfirmation(false)}
        >{$t("settings.cancel")}</button
      ><button
        type="button"
        class="danger-button"
        disabled={clearDisabled || clearing}
        onclick={() => void clear()}
        >{$t("settings.clear_history_confirm")}</button
      >
    </div>
    {#if clearError}<p class="field-error" role="alert">
        {$text(clearError)}
      </p>{/if}
  </div>{:else}<button
    id="clear-history"
    type="button"
    class="secondary-button"
    bind:this={clearButton}
    disabled={clearDisabled || clearing}
    onclick={() => void showClearConfirmation(true)}
    >{$t("settings.clear_history_action")}</button
  >{/if}
{#if feedback}<p class="settings-feedback" role="status">
    {$text(feedback)}
  </p>{/if}
