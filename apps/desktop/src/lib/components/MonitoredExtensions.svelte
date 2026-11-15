<script lang="ts">
  import { t } from "../i18n";
  import type {
    ExtensionState,
    MonitoredExtensionSnapshot,
  } from "../generated/AppInfo";

  let {
    extensions,
    canDial,
    oncall,
  }: {
    extensions: MonitoredExtensionSnapshot[];
    canDial: boolean;
    oncall: (extension: string) => void;
  } = $props();
  const stateKeys = {
    unknown: "phone.blf.unknown",
    idle: "phone.blf.idle",
    ringing: "phone.blf.ringing",
    busy: "phone.blf.busy",
  } as const satisfies Record<ExtensionState, string>;
</script>

{#if extensions.length}
  <section class="monitored-extensions" aria-label={$t("phone.blf.title")}>
    <h2>{$t("phone.blf.title")}</h2>
    <ul>
      {#each extensions as target (target.extension)}
        <li>
          <button
            type="button"
            disabled={!canDial}
            aria-label={$t("phone.blf.call", {
              name: target.name || target.extension,
              extension: target.extension,
              state: $t(stateKeys[target.state]),
            })}
            onclick={() => {
              if (canDial) oncall(target.extension);
            }}
          >
            <span class="extension-name">
              <span>{target.name || target.extension}</span>
              {#if target.name}<span class="extension-number"
                  >{target.extension}</span
                >{/if}
            </span>
            <span class="extension-state" data-state={target.state}>
              <span class="state-dot" aria-hidden="true"></span>
              {$t(stateKeys[target.state])}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .monitored-extensions {
    margin-top: 16px;
    max-height: min(240px, 40vh);
    overflow-y: auto;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    padding: 12px 0 0;
  }
  h2 {
    margin: 0 0 6px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    min-height: 36px;
    border: 0;
    border-radius: var(--radius);
    padding: 6px 8px;
    background: transparent;
    color: var(--text);
    text-align: left;
    font-size: 13px;
  }
  button:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  button:disabled {
    cursor: default;
  }
  .extension-name {
    display: flex;
    min-width: 0;
    gap: 8px;
    align-items: baseline;
    overflow-wrap: anywhere;
  }
  .extension-number {
    color: var(--muted);
    font-size: 12px;
  }
  .extension-state {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .state-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
  }
  [data-state="idle"] .state-dot {
    background: var(--accent);
  }
  [data-state="ringing"] .state-dot {
    background: var(--caution);
  }
  [data-state="busy"] .state-dot {
    background: var(--danger);
  }
</style>
