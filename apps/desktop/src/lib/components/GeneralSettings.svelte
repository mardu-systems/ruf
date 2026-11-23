<script lang="ts">
  import { t, text, type MessageValue } from "../i18n";
  import type { LanguagePreference } from "../generated/AppInfo";
  let {
    autostart = $bindable(),
    startHidden = $bindable(),
    language = $bindable("system"),
    available,
    unknownValues = null,
    disabled,
    dirty,
    onsave,
    ondiscard,
  }: {
    autostart: boolean;
    startHidden: boolean;
    language: LanguagePreference;
    available: boolean;
    unknownValues?: MessageValue | null;
    disabled: boolean;
    dirty: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.general")}</h1>
    <p>{$t("settings.general_description")}</p>
  </div>
</div>
{#if unknownValues}
  <p class="settings-footnote">{$text(unknownValues)}</p>
{:else}<fieldset {disabled}>
    <h2 class="settings-group">{$t("settings.appearance")}</h2>
    <div class="setting-row">
      <div class="setting-description">
        <label for="language-preference">{$t("settings.language")}</label>
        <p id="language-help">{$t("settings.language_help")}</p>
      </div>
      <div class="setting-control">
        <select
          id="language-preference"
          bind:value={language}
          aria-describedby="language-help"
        >
          <option value="system">{$t("settings.language_system")}</option>
          <option value="de">Deutsch</option>
          <option value="en">English</option>
        </select>
      </div>
    </div>
    <h2 class="settings-group sound-group">
      {$t("settings.startup_background")}
    </h2>
    <div class="setting-row">
      <div class="setting-description">
        <label for="autostart-enabled">{$t("settings.autostart")}</label>
        {#if !available}<p>
            {$t("settings.autostart_unavailable")}
          </p>{/if}
      </div>
      <div class="setting-control sound-toggle">
        <input
          id="autostart-enabled"
          type="checkbox"
          bind:checked={autostart}
          disabled={!available}
        /><span aria-hidden="true"
          >{autostart ? $t("settings.on") : $t("settings.off")}</span
        >
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="start-hidden">{$t("settings.start_hidden")}</label>
        <p>
          {$t("settings.start_hidden_help")}
        </p>
      </div>
      <div class="setting-control sound-toggle">
        <input
          id="start-hidden"
          type="checkbox"
          bind:checked={startHidden}
        /><span aria-hidden="true"
          >{startHidden ? $t("settings.on") : $t("settings.off")}</span
        >
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <h3>{$t("settings.stay_available")}</h3>
        <p>
          {$t("settings.stay_available_help")}
        </p>
      </div>
    </div>
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
          disabled={!dirty}
          onclick={onsave}>{$t("settings.save_general")}</button
        >
      </div>
    </div>
  </fieldset>{/if}
