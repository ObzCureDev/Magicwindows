<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { appState } from "../lib/stores.svelte";
  import { t } from "../lib/i18n";
  import type { ModifierState, ModifierToggles, RawScancodePair } from "../lib/types";

  let modState = $state<ModifierState | null>(null);
  let loading = $state(true);
  let toggles = $state<ModifierToggles>({
    swapCmdCtrlLeft: false,
    swapCmdCtrlRight: false,
    capsToCtrl: false,
    swapOptionCmd: false,
  });
  let phase = $state<"select" | "preview" | "applying">("select");
  let error = $state<string | null>(null);
  let success = $state(false);
  let showExternalDetails = $state(false);

  // The "both sides" UI checkbox is a derived view of the two per-side flags.
  let bothSides = $derived(toggles.swapCmdCtrlLeft && toggles.swapCmdCtrlRight);
  let cmdCtrlActive = $derived(toggles.swapCmdCtrlLeft || toggles.swapCmdCtrlRight);

  async function load() {
    loading = true;
    error = null;
    try {
      const s = await invoke<ModifierState>("read_scancode_map");
      modState = s;
      toggles = { ...s.current };
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function setBothSides(v: boolean) {
    toggles.swapCmdCtrlLeft  = v;
    toggles.swapCmdCtrlRight = v;
    if (v) toggles.swapOptionCmd = false; // mutual exclusion
  }

  function setSwapOptionCmd(v: boolean) {
    toggles.swapOptionCmd = v;
    if (v) {
      toggles.swapCmdCtrlLeft  = false;
      toggles.swapCmdCtrlRight = false;
    }
  }

  function toPreview() {
    error = null;
    phase = "preview";
  }

  async function apply() {
    phase = "applying";
    error = null;
    try {
      await invoke("write_scancode_map", { toggles });
      success = true;
      await load(); // refresh state from registry
      phase = "select";
    } catch (e) {
      error = String(e);
      phase = "preview";
    }
  }

  async function disableAll() {
    phase = "applying";
    error = null;
    try {
      await invoke("clear_scancode_map");
      success = true;
      await load();
      phase = "select";
    } catch (e) {
      error = String(e);
      phase = "select";
    }
  }

  function back() {
    appState.page = "welcome";
  }

  // Helpers for the preview panel
  function pairsForCurrent(): RawScancodePair[] {
    return modState?.rawEntries ?? [];
  }
  function describePair(p: RawScancodePair): string {
    const labels: Record<string, string> = {
      "1D00": "LCtrl",
      "1DE0": "RCtrl",
      "5BE0": "LWin (Cmd)",
      "5CE0": "RWin (Cmd)",
      "3800": "LAlt (Option)",
      "38E0": "RAlt (Option)",
      "3A00": "CapsLock",
    };
    const o = labels[p.oldCode] ?? p.oldCode;
    const n = labels[p.newCode] ?? p.newCode;
    return `${o} → ${n}`;
  }

  // Local mirror of the Rust build_scancode_map for the preview panel.
  function pair(newCode: string, oldCode: string): RawScancodePair {
    return { newCode, oldCode };
  }
  function previewPairs(t: ModifierToggles): RawScancodePair[] {
    const r: RawScancodePair[] = [];
    if (t.swapCmdCtrlLeft) {
      r.push(pair("1D00", "5BE0"));
      r.push(pair("5BE0", "1D00"));
    }
    if (t.swapCmdCtrlRight) {
      r.push(pair("1DE0", "5CE0"));
      r.push(pair("5CE0", "1DE0"));
    }
    if (t.capsToCtrl) r.push(pair("1D00", "3A00"));
    if (t.swapOptionCmd) {
      r.push(pair("3800", "5BE0"));
      r.push(pair("5BE0", "3800"));
      r.push(pair("38E0", "5CE0"));
      r.push(pair("5CE0", "38E0"));
    }
    return r;
  }

  onMount(load);
</script>

<div class="page">
  <div class="page__header">
    <h1 class="page__title">{t(appState.lang, "modifiers.title")}</h1>
    <p class="page__subtitle">{t(appState.lang, "modifiers.description")}</p>
  </div>

  <div class="page__content">
    {#if loading}
      <div class="spinner"></div>
    {:else if phase === "select"}
      {#if modState?.hasExternalMappings}
        <div class="status status--warning" role="alert">
          {t(appState.lang, "modifiers.externalWarning")}
          <button class="link" onclick={() => (showExternalDetails = !showExternalDetails)}>
            {t(appState.lang, "modifiers.externalDetails")}
          </button>
          {#if showExternalDetails && modState}
            <ul class="raw-pairs">
              {#each modState.rawEntries as p}
                <li><code>{describePair(p)}</code></li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      {#if success}
        <div class="status status--success">{t(appState.lang, "modifiers.applied")}</div>
      {/if}

      <div class="toggle-list">
        <label class="toggle-row">
          <input type="checkbox"
                 checked={bothSides}
                 disabled={toggles.swapOptionCmd}
                 onchange={(e) => setBothSides((e.currentTarget as HTMLInputElement).checked)} />
          <div>
            <div class="toggle-label">{t(appState.lang, "modifiers.toggleSwapBoth")}</div>
            <div class="toggle-hint">{t(appState.lang, "modifiers.toggleSwapBothHint")}</div>
          </div>
        </label>

        <label class="toggle-row">
          <input type="checkbox"
                 bind:checked={toggles.swapCmdCtrlLeft}
                 disabled={toggles.swapOptionCmd} />
          <div class="toggle-label">{t(appState.lang, "modifiers.toggleSwapLeft")}</div>
        </label>

        <label class="toggle-row">
          <input type="checkbox"
                 bind:checked={toggles.swapCmdCtrlRight}
                 disabled={toggles.swapOptionCmd} />
          <div class="toggle-label">{t(appState.lang, "modifiers.toggleSwapRight")}</div>
        </label>

        <label class="toggle-row">
          <input type="checkbox" bind:checked={toggles.capsToCtrl} />
          <div>
            <div class="toggle-label">{t(appState.lang, "modifiers.toggleCaps")}</div>
            <div class="toggle-hint">{t(appState.lang, "modifiers.toggleCapsHint")}</div>
          </div>
        </label>

        <label class="toggle-row">
          <input type="checkbox"
                 checked={toggles.swapOptionCmd}
                 disabled={cmdCtrlActive}
                 onchange={(e) => setSwapOptionCmd((e.currentTarget as HTMLInputElement).checked)} />
          <div>
            <div class="toggle-label">{t(appState.lang, "modifiers.toggleOptionCmd")}</div>
            <div class="toggle-hint">{t(appState.lang, "modifiers.toggleOptionCmdHint")}</div>
          </div>
        </label>
      </div>

      {#if error}<div class="status status--error">{error}</div>{/if}

      <div class="page__actions">
        <button class="btn btn-primary" onclick={toPreview}>
          {t(appState.lang, "modifiers.preview")}
        </button>
        <button class="btn btn-danger" onclick={disableAll}>
          {t(appState.lang, "modifiers.disableAll")}
        </button>
        <button class="btn btn-secondary" onclick={back}>
          {t(appState.lang, "modifiers.back")}
        </button>
      </div>

    {:else if phase === "preview" || phase === "applying"}
      <h2>{t(appState.lang, "modifiers.previewTitle")}</h2>

      <div class="preview-grid">
        <div>
          <h3>{t(appState.lang, "modifiers.previewBefore")}</h3>
          {#if pairsForCurrent().length === 0}
            <p class="text-secondary">{t(appState.lang, "modifiers.previewNoChange")}</p>
          {:else}
            <ul class="raw-pairs">
              {#each pairsForCurrent() as p}<li><code>{describePair(p)}</code></li>{/each}
            </ul>
          {/if}
        </div>

        <div>
          <h3>{t(appState.lang, "modifiers.previewAfter")}</h3>
          {#if previewPairs(toggles).length === 0}
            <p class="text-secondary">{t(appState.lang, "modifiers.previewNoChange")}</p>
          {:else}
            <ul class="raw-pairs">
              {#each previewPairs(toggles) as p}<li><code>{describePair(p)}</code></li>{/each}
            </ul>
          {/if}
        </div>
      </div>

      <div class="status status--warning">{t(appState.lang, "modifiers.rebootWarning")}</div>

      {#if error}<div class="status status--error">{error}</div>{/if}

      <div class="page__actions">
        <button class="btn btn-primary"
                disabled={phase === "applying"}
                onclick={apply}>
          {phase === "applying" ? t(appState.lang, "modifiers.applying") : t(appState.lang, "modifiers.apply")}
        </button>
        <button class="btn btn-secondary"
                disabled={phase === "applying"}
                onclick={() => (phase = "select")}>
          {t(appState.lang, "modifiers.cancel")}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .toggle-list { display: flex; flex-direction: column; gap: 12px; max-width: 560px; margin: 1rem auto; }
  .toggle-row {
    display: flex; align-items: flex-start; gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--color-border); border-radius: var(--radius-md);
    background: var(--color-bg-card);
    cursor: pointer;
  }
  .toggle-row input[type="checkbox"] { margin-top: 3px; }
  .toggle-label { font-weight: 500; color: var(--color-text); }
  .toggle-hint  { font-size: 13px; color: var(--color-text-secondary); margin-top: 2px; }
  .preview-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; max-width: 720px; margin: 1rem auto; }
  .raw-pairs { list-style: none; padding: 0; margin: 0; }
  .raw-pairs li { padding: 4px 0; }
  .raw-pairs code { font-family: var(--font-mono); font-size: 13px; }
  .link { background: none; border: none; color: var(--color-accent); text-decoration: underline; cursor: pointer; padding: 0; margin-left: 8px; }
</style>
