<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { appState } from "../lib/stores.svelte";
  import { t } from "../lib/i18n";
  import type { DetectionKey, DetectionResult } from "../lib/types";

  let detectionKeys = $state<DetectionKey[]>([]);
  let currentStep = $state(0);
  let results = $state<DetectionResult[]>([]);
  let analyzing = $state(false);
  let detectedId = $state<string | null>(null);
  let detectedName = $state<string>("");
  let failed = $state(false);
  let waiting = $state(true);

  // Load detection keys from the first available layout (they share detection keys)
  onMount(async () => {
    try {
      const layoutIds = appState.layouts.map((l) => l.id);
      const keys = await invoke<DetectionKey[]>("get_detection_keys", { layoutIds });
      detectionKeys = keys;
    } catch (err) {
      console.error("Failed to get detection keys:", err);
      appState.error = String(err);
    } finally {
      waiting = false;
    }
  });

  let totalKeys = $derived(detectionKeys.length);
  let currentKey = $derived(detectionKeys[currentStep] ?? null);
  let progress = $derived(totalKeys > 0 ? ((currentStep) / totalKeys) * 100 : 0);

  function handleKeydown(event: KeyboardEvent) {
    if (analyzing || failed || detectedId || !currentKey) return;

    // Only capture if this is the expected event code
    if (event.code === currentKey.eventCode) {
      event.preventDefault();

      const result: DetectionResult = {
        eventCode: event.code,
        receivedChar: event.key,
      };
      results = [...results, result];

      if (currentStep + 1 < totalKeys) {
        currentStep += 1;
      } else {
        runDetection();
      }
    }
  }

  async function runDetection() {
    analyzing = true;
    try {
      const matchId = await invoke<string | null>("match_detection", {
        results: results,
      });
      if (matchId) {
        detectedId = matchId;
        appState.selectedLayoutId = matchId;
        // Find the name
        const layout = appState.layouts.find((l) => l.id === matchId);
        detectedName = layout?.name[appState.lang] ?? matchId;
      } else {
        failed = true;
      }
    } catch (err) {
      console.error("Detection matching failed:", err);
      failed = true;
    } finally {
      analyzing = false;
    }
  }

  function retry() {
    currentStep = 0;
    results = [];
    detectedId = null;
    detectedName = "";
    failed = false;
    analyzing = false;
  }

  function goPreview() {
    if (detectedId) {
      appState.page = "preview";
    }
  }

  function goSelect() {
    appState.page = "select";
  }

  function goBack() {
    appState.page = "welcome";
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<div class="page">
  <div class="page__header">
    <h1 class="page__title">{t(appState.lang, "detect.title")}</h1>
    <p class="page__subtitle">{t(appState.lang, "detect.instruction")}</p>
  </div>

  <div class="page__content">
    {#if waiting}
      <div class="spinner"></div>
      <p class="text-secondary">{t(appState.lang, "common.loading")}</p>
    {:else if analyzing}
      <div class="spinner"></div>
      <p class="text-secondary">{t(appState.lang, "detect.analyzing")}</p>
    {:else if detectedId}
      <div class="status status--success">
        {t(appState.lang, "detect.detected", { name: detectedName })}
      </div>
      <div class="page__actions">
        <button class="btn btn-primary" onclick={goPreview}>
          {t(appState.lang, "detect.continue")}
        </button>
      </div>
    {:else if failed}
      <div class="status status--error">
        {t(appState.lang, "detect.notDetected")}
      </div>
      <div class="page__actions">
        <button class="btn btn-secondary" onclick={retry}>
          {t(appState.lang, "detect.tryAgain")}
        </button>
        <button class="btn btn-primary" onclick={goSelect}>
          {t(appState.lang, "detect.fallback")}
        </button>
      </div>
    {:else if currentKey}
      <div class="progress-bar" role="progressbar" aria-valuenow={Math.round(progress)} aria-valuemin={0} aria-valuemax={100}>
        <div class="progress-bar__fill" style="width: {progress}%"></div>
      </div>
      <p class="text-secondary">
        {t(appState.lang, "detect.progress", {
          current: String(currentStep + 1),
          total: String(totalKeys),
        })}
      </p>

      <div class="detect-prompt">
        <p class="detect-prompt__text">{t(appState.lang, "detect.pressKey")}</p>
        <p class="detect-prompt__key-name">
          {currentKey.prompt[appState.lang] ?? currentKey.prompt["en"] ?? currentKey.eventCode}
        </p>
      </div>

      <!-- Apple Magic Keyboard ISO – 1:1 replica -->
      <div class="mk-body">
        <!-- Row 0: Function row + Touch ID -->
        <div class="mk-row mk-row--fn">
          <div class="mk-key mk-key--fn-esc"><span class="mk-lbl-sm">esc</span></div>

          <div class="mk-key mk-key--fn" title="F1 — Brightness Down">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="1.8" fill="none" stroke="currentColor" stroke-width="1.1"/>
              <g stroke="currentColor" stroke-width="1.1" stroke-linecap="round">
                <line x1="8" y1="3.6" x2="8" y2="4.8"/><line x1="8" y1="11.2" x2="8" y2="12.4"/>
                <line x1="3.6" y1="8" x2="4.8" y2="8"/><line x1="11.2" y1="8" x2="12.4" y2="8"/>
                <line x1="4.9" y1="4.9" x2="5.7" y2="5.7"/><line x1="10.3" y1="10.3" x2="11.1" y2="11.1"/>
                <line x1="11.1" y1="4.9" x2="10.3" y2="5.7"/><line x1="5.7" y1="10.3" x2="4.9" y2="11.1"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F1</span>
          </div>

          <div class="mk-key mk-key--fn" title="F2 — Brightness Up">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="2.4" fill="none" stroke="currentColor" stroke-width="1.1"/>
              <g stroke="currentColor" stroke-width="1.1" stroke-linecap="round">
                <line x1="8" y1="2.5" x2="8" y2="4.4"/><line x1="8" y1="11.6" x2="8" y2="13.5"/>
                <line x1="2.5" y1="8" x2="4.4" y2="8"/><line x1="11.6" y1="8" x2="13.5" y2="8"/>
                <line x1="4.1" y1="4.1" x2="5.4" y2="5.4"/><line x1="10.6" y1="10.6" x2="11.9" y2="11.9"/>
                <line x1="11.9" y1="4.1" x2="10.6" y2="5.4"/><line x1="5.4" y1="10.6" x2="4.1" y2="11.9"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F2</span>
          </div>

          <div class="mk-key mk-key--fn" title="F3 — Mission Control">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="none" stroke="currentColor" stroke-width="1">
                <rect x="2.5" y="3" width="4.5" height="3" rx="0.5"/>
                <rect x="9" y="3" width="4.5" height="3" rx="0.5"/>
                <rect x="2.5" y="8" width="4.5" height="5" rx="0.5"/>
                <rect x="9" y="8" width="4.5" height="5" rx="0.5"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F3</span>
          </div>

          <div class="mk-key mk-key--fn" title="F4 — Spotlight">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
                <circle cx="6.8" cy="6.8" r="3.4"/>
                <line x1="9.4" y1="9.4" x2="13" y2="13"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F4</span>
          </div>

          <div class="mk-key mk-key--fn" title="F5 — Dictation">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round">
                <rect x="6" y="2.5" width="4" height="6.5" rx="2"/>
                <path d="M4 8 a4 4 0 0 0 8 0"/>
                <line x1="8" y1="12" x2="8" y2="13.5"/>
                <line x1="6.5" y1="13.5" x2="9.5" y2="13.5"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F5</span>
          </div>

          <div class="mk-key mk-key--fn" title="F6 — Do Not Disturb">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M12 10.5 A5 5 0 1 1 6.2 3 a4 4 0 0 0 5.8 7.5 z"
                    fill="none" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
            </svg>
            <span class="mk-lbl-fn">F6</span>
          </div>

          <div class="mk-key mk-key--fn" title="F7 — Previous">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="currentColor">
                <rect x="2.5" y="4.5" width="1.3" height="7" rx="0.3"/>
                <polygon points="13.5,4.5 13.5,11.5 8.5,8"/>
                <polygon points="8.5,4.5 8.5,11.5 4,8"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F7</span>
          </div>

          <div class="mk-key mk-key--fn" title="F8 — Play / Pause">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="currentColor">
                <polygon points="2.5,3.5 2.5,12.5 8,8"/>
                <rect x="9.5" y="3.5" width="1.5" height="9" rx="0.3"/>
                <rect x="12" y="3.5" width="1.5" height="9" rx="0.3"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F8</span>
          </div>

          <div class="mk-key mk-key--fn" title="F9 — Next">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="currentColor">
                <polygon points="2.5,4.5 2.5,11.5 7.5,8"/>
                <polygon points="7.5,4.5 7.5,11.5 12,8"/>
                <rect x="12.2" y="4.5" width="1.3" height="7" rx="0.3"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F9</span>
          </div>

          <div class="mk-key mk-key--fn" title="F10 — Mute">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <polygon points="2,6.2 5,6.2 8.5,3.5 8.5,12.5 5,9.8 2,9.8" fill="currentColor"/>
              <g fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round">
                <line x1="11" y1="6" x2="14" y2="10"/>
                <line x1="14" y1="6" x2="11" y2="10"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F10</span>
          </div>

          <div class="mk-key mk-key--fn" title="F11 — Volume Down">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <polygon points="2,6.2 5,6.2 8.5,3.5 8.5,12.5 5,9.8 2,9.8" fill="currentColor"/>
              <path d="M10.8 6.2 a3 3 0 0 1 0 3.6"
                    fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
            </svg>
            <span class="mk-lbl-fn">F11</span>
          </div>

          <div class="mk-key mk-key--fn" title="F12 — Volume Up">
            <svg class="mk-fn-icon" viewBox="0 0 16 16" aria-hidden="true">
              <polygon points="2,6.2 5,6.2 8.5,3.5 8.5,12.5 5,9.8 2,9.8" fill="currentColor"/>
              <g fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round">
                <path d="M10.8 6.2 a3 3 0 0 1 0 3.6"/>
                <path d="M12.6 4.5 a5 5 0 0 1 0 7"/>
              </g>
            </svg>
            <span class="mk-lbl-fn">F12</span>
          </div>

          <div class="mk-touchid" title="Touch ID"></div>
        </div>

        <!-- Row 1: Number row + Delete -->
        <div class="mk-row">
          {#each ["Backquote", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9", "Digit0", "Minus", "Equal"] as code}
            <div class="mk-key" class:mk-key--active={currentKey.eventCode === code}></div>
          {/each}
          <div class="mk-key mk-key--delete"><span class="mk-lbl-sm">delete</span></div>
        </div>

        <!-- Row 2: Tab + letters + Enter top stub -->
        <div class="mk-row">
          <div class="mk-key mk-key--tab"><span class="mk-lbl-sm">tab</span></div>
          {#each ["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP", "BracketLeft", "BracketRight"] as code}
            <div class="mk-key" class:mk-key--active={currentKey.eventCode === code}></div>
          {/each}
          <div class="mk-key mk-key--enter-top" class:mk-key--active={currentKey.eventCode === 'Enter'}></div>
        </div>

        <!-- Row 3: Caps + letters + Enter bottom -->
        <div class="mk-row">
          <div class="mk-key mk-key--caps"><span class="mk-lbl-sm">caps lock</span></div>
          {#each ["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL", "Semicolon", "Quote", "Backslash"] as code}
            <div class="mk-key" class:mk-key--active={currentKey.eventCode === code}></div>
          {/each}
          <div class="mk-key mk-key--enter-bot" class:mk-key--active={currentKey.eventCode === 'Enter'}>
            <span class="mk-lbl-sm">return</span>
          </div>
        </div>

        <!-- Row 4: Shifts + bottom letters -->
        <div class="mk-row">
          <div class="mk-key mk-key--lshift"><span class="mk-lbl-sm">shift</span></div>
          {#each ["IntlBackslash", "KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM", "Comma", "Period", "Slash"] as code}
            <div class="mk-key" class:mk-key--active={currentKey.eventCode === code}></div>
          {/each}
          <div class="mk-key mk-key--rshift"><span class="mk-lbl-sm">shift</span></div>
        </div>

        <!-- Row 5: Modifiers + Space + Arrows -->
        <div class="mk-row mk-row--bottom">
          <div class="mk-key mk-key--mod1" title="Globe / Function">
            <svg class="mk-globe" viewBox="0 0 16 16" aria-hidden="true">
              <g fill="none" stroke="currentColor" stroke-width="1.1">
                <circle cx="8" cy="8" r="5.3"/>
                <ellipse cx="8" cy="8" rx="2.1" ry="5.3"/>
                <line x1="2.7" y1="8" x2="13.3" y2="8"/>
                <path d="M3.3 5 Q8 6.5 12.7 5"/>
                <path d="M3.3 11 Q8 9.5 12.7 11"/>
              </g>
            </svg>
          </div>
          <div class="mk-key mk-key--mod1">
            <span class="mk-glyph">⌃</span>
            <span class="mk-lbl-xs">control</span>
          </div>
          <div class="mk-key mk-key--mod1">
            <span class="mk-glyph">⌥</span>
            <span class="mk-lbl-xs">option</span>
          </div>
          <div class="mk-key mk-key--cmd">
            <span class="mk-glyph">⌘</span>
            <span class="mk-lbl-xs">command</span>
          </div>
          <div class="mk-key mk-key--space" class:mk-key--active={currentKey.eventCode === 'Space'}></div>
          <div class="mk-key mk-key--cmd">
            <span class="mk-glyph">⌘</span>
            <span class="mk-lbl-xs">command</span>
          </div>
          <div class="mk-key mk-key--mod1">
            <span class="mk-glyph">⌥</span>
            <span class="mk-lbl-xs">option</span>
          </div>
          <div class="mk-arrows">
            <div class="mk-arrow mk-arrow--l"><span class="mk-arrow-glyph">◀</span></div>
            <div class="mk-arrow-stack">
              <div class="mk-arrow mk-arrow--h"><span class="mk-arrow-glyph">▲</span></div>
              <div class="mk-arrow mk-arrow--h"><span class="mk-arrow-glyph">▼</span></div>
            </div>
            <div class="mk-arrow mk-arrow--l"><span class="mk-arrow-glyph">▶</span></div>
          </div>
        </div>
      </div>
    {:else}
      <div class="status status--error">
        {t(appState.lang, "detect.notDetected")}
      </div>
    {/if}

    <div class="page__actions">
      <button class="btn btn-secondary" onclick={goBack}>
        {t(appState.lang, "detect.back")}
      </button>
    </div>
  </div>
</div>
