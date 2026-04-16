# Mac-Style Modifier Keys (Scancode Swap)

**Date:** 2026-04-17
**Status:** Approved — ready for implementation plan
**Adds to:** existing keyboard layout install flow

## Problem

When the user installs a layout via MagicWindows, character keys produce the right symbols, but the modifier keys still behave like a Windows keyboard:

- The Apple "command" key acts as the Windows key — so Cmd+C opens Copilot instead of copying.
- The Apple "control" key acts as Ctrl, so the user has to retrain themselves to use it for shortcuts.

Mac users want their muscle memory back: Cmd+C copies, Cmd+V pastes, Cmd+W closes, etc.

## Solution

A separate page in the app that lets the user toggle individual scancode-level modifier remappings. Selections are previewed before being written to `HKLM\System\CurrentControlSet\Control\Keyboard Layout\Scancode Map` as a single binary value.

This is independent of the keyboard layout flow — modifier remapping is system-wide and survives layout switches.

## The toggles

Each toggle is independent and adds one or more entries to the Scancode Map binary. The user can mix freely.

| Toggle | Effect | Scancode pairs added |
|--------|--------|----------------------|
| **Swap Cmd ↔ Ctrl (both sides)** | Mac muscle memory for shortcuts: Cmd+C copies, the "control" key becomes Win for system shortcuts. | LCtrl ↔ LWin, RCtrl ↔ RWin |
| **Swap Cmd ↔ Ctrl (left only)** | Same as above but only on the left side. Right Cmd stays as Win, useful if the user maps right-Cmd to specialized shortcuts. | LCtrl ↔ LWin |
| **Swap Cmd ↔ Ctrl (right only)** | Mirror of left-only. | RCtrl ↔ RWin |
| **Caps Lock → Ctrl** | Adds an extra Ctrl key under the left pinky. Popular among Vim/Emacs users and people who already remap Caps Lock on macOS. | CapsLock → LCtrl (one-way) |
| **Swap Option ↔ Cmd (positions Mac strict)** | Strictly mirrors the Mac key order from spacebar outward: Cmd, Option, Ctrl. Without this, the post-Cmd↔Ctrl order is Ctrl, Option, Win which differs from Mac. | LAlt ↔ LWin (after Cmd↔Ctrl swap), RAlt ↔ RWin |

The "left only" / "right only" Cmd↔Ctrl toggles are mutually exclusive with the "both sides" toggle — selecting both sides un-checks the per-side ones automatically (and vice versa).

## Scancode Map binary format

Standard Microsoft format:

```
00 00 00 00     ; header version (always 0)
00 00 00 00     ; header flags   (always 0)
NN 00 00 00     ; entry count, including the null terminator
<entries>       ; each entry = 4 bytes: 2-byte new scancode + 2-byte old scancode
00 00 00 00     ; null terminator
```

Scancode reference for our toggles:

| Key | Scancode (low/high, little-endian bytes) |
|-----|------------------------------------------|
| LCtrl | `1D 00` |
| LWin | `5B E0` |
| LAlt | `38 00` |
| RCtrl | `1D E0` |
| RWin | `5C E0` |
| RAlt | `38 E0` |
| CapsLock | `3A 00` |

Example: full Cmd↔Ctrl swap (both sides) emits four entries:

```
1D 00  5B E0     ; pressing LWin → emit LCtrl
5B E0  1D 00     ; pressing LCtrl → emit LWin
1D E0  5C E0     ; pressing RWin → emit RCtrl
5C E0  1D E0     ; pressing RCtrl → emit RWin
```

Reads as `<NEW> <OLD>` per entry: when the OLD scancode is received, the OS reports the NEW one.

## UI / UX

A new page `Modifiers` (FR: "Modificateurs"), reachable from a new icon button in the top bar (next to theme toggle). Two-step workflow within the page:

### Step 1 — Select

```
┌──────────────────────────────────────────────────┐
│  Modificateurs Mac                                │
│                                                  │
│  Détectées : [ aucune | swap Cmd↔Ctrl actif ]    │
│                                                  │
│  ☐ Échanger Cmd ↔ Ctrl (les deux côtés)          │
│      Cmd+C copie, Cmd+V colle, etc.              │
│                                                  │
│  ☐ Échanger Cmd ↔ Ctrl (gauche seulement)        │
│  ☐ Échanger Cmd ↔ Ctrl (droite seulement)        │
│                                                  │
│  ☐ Caps Lock → Ctrl supplémentaire               │
│      Pratique pour Vim/Emacs.                    │
│                                                  │
│  ☐ Échanger Option ↔ Cmd (positions Mac strict)  │
│                                                  │
│  [ Aperçu ]   [ Désactiver tout ]   [ Retour ]   │
└──────────────────────────────────────────────────┘
```

On load, the page reads the current Scancode Map and pre-checks any of our recognized mappings. Unknown mappings (from SharpKeys / PowerToys / etc.) trigger an inline warning banner above the toggles:

> ⚠ Mappings externes détectés. Cliquer "Appliquer" remplacera les vôtres et ceux d'autres outils. [Voir détails]

### Step 2 — Preview & Apply

Clicking "Aperçu" opens a confirmation panel inline (or modal, TBD by implementation):

```
┌──────────────────────────────────────────────────┐
│  Aperçu des changements                          │
│                                                  │
│  Avant :                                         │
│  ─────────                                       │
│  • LWin → (défaut Windows)                       │
│  • LCtrl → (défaut Windows)                      │
│  • CapsLock → (défaut Windows)                   │
│                                                  │
│  Après :                                         │
│  ─────────                                       │
│  • LWin → LCtrl (Cmd devient Ctrl)               │
│  • LCtrl → LWin (Control devient Win)            │
│  • RWin → RCtrl                                  │
│  • RCtrl → RWin                                  │
│  • CapsLock → LCtrl                              │
│                                                  │
│  ⚠ Un redémarrage est requis pour activer.       │
│                                                  │
│  [ Annuler ]   [ Appliquer maintenant ]          │
└──────────────────────────────────────────────────┘
```

"Appliquer maintenant" → UAC prompt → Rust writes the binary to HKLM → success toast → optional "Redémarrer maintenant" button.

"Tout désactiver" deletes the `Scancode Map` value entirely (also UAC-gated, also reboot-required, also previewed).

### Conflict handling (warn, not block)

If the existing `Scancode Map` value contains entries we didn't write (i.e., entries that don't correspond to any of our 5 toggles' canonical pairs), surface a warning banner BEFORE the user even checks toggles:

> ⚠ Des remappages clavier externes ont été détectés (SharpKeys, PowerToys, …). Si vous appliquez vos modifications ici, les leurs seront remplacés. Continuer ?

The user can dismiss and proceed. We don't try to merge — too risky and the user may not even know what those mappings are.

## Architecture

### Frontend

- New route/page `src/pages/Modifiers.svelte`
- Add `"modifiers"` to the `Page` union in `src/lib/types.ts`
- Top-bar icon button to navigate to it (placed next to the theme toggle in `src/App.svelte`)
- Local component state for the toggle checkboxes; computed `pendingScancodeMap` derived from toggles

### Backend (Rust)

Three new Tauri commands in `src-tauri/src/keyboard/modifiers.rs` (new file):

```rust
#[tauri::command]
fn read_scancode_map() -> Result<ModifierState, String>;

#[tauri::command]
fn write_scancode_map(toggles: ModifierToggles) -> Result<(), String>;

#[tauri::command]
fn clear_scancode_map() -> Result<(), String>;
```

Types:

```rust
#[derive(Serialize, Deserialize)]
pub struct ModifierToggles {
    pub swap_cmd_ctrl_left: bool,
    pub swap_cmd_ctrl_right: bool,
    pub caps_to_ctrl: bool,
    pub swap_option_cmd: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ModifierState {
    pub current: ModifierToggles,        // best-fit reverse-derivation from registry
    pub has_external_mappings: bool,     // entries we don't recognize
    pub raw_entries: Vec<RawScancodePair>,
}

#[derive(Serialize, Deserialize)]
pub struct RawScancodePair {
    pub new_code: String,  // hex like "1D00"
    pub old_code: String,
}
```

Read/write goes through PowerShell (consistent with the existing layout-install pattern) — easier than wrestling with `winreg` for binary REG_BINARY values, and keeps elevation handling in one place.

### Reverse-derivation logic

To pre-check the toggles based on existing Scancode Map content:

1. Parse the binary into list of (new, old) pairs.
2. For each toggle's canonical pairs, check if **all** of its pairs are present.
3. If yes → toggle is on. If only some are present → toggle is off but `has_external_mappings = true`.

The "swap Cmd↔Ctrl both sides" toggle is recognized when both LCtrl↔LWin and RCtrl↔RWin pairs are present. The per-side toggles are recognized only when EXACTLY their side's pair exists (not the other).

### Mutual exclusion in the UI

If the user checks "both sides", uncheck "left only" and "right only".
If the user checks one side, uncheck "both sides" (and the other side stays as the user left it).

## Implementation rules

1. **Always preview before apply.** Never write to HKLM directly from a checkbox change.
2. **UAC happens once at apply time** — the elevated PowerShell does the registry write.
3. **Reboot warning** is non-dismissible in the preview step (it's a visual element, not a checkbox).
4. **Disable-all** clears the value entirely; doesn't try to surgically remove only our entries (cleaner, simpler).
5. **Per-side / both-sides toggles** are mutually exclusive in the UI — the union of selections is what gets written.
6. **No partial application.** If the registry write fails for any reason, leave the existing Scancode Map untouched. This means the elevated PS computes the new binary, validates it, then writes — no incremental edits.
7. **Empty mapping == delete the value.** If the user unchecks everything and applies, we delete `Scancode Map` rather than write a header-only binary (some Windows builds get confused by the empty entry-count form).

## Files to change

| File | Status | Responsibility |
|------|--------|----------------|
| `src-tauri/src/keyboard/modifiers.rs` | NEW | Scancode-map read/write + canonical-pair detection logic |
| `src-tauri/src/keyboard/mod.rs` | modify | Re-export the new module |
| `src-tauri/src/lib.rs` | modify | Register the 3 new Tauri commands |
| `src/pages/Modifiers.svelte` | NEW | Two-step UI (select + preview & apply) |
| `src/App.svelte` | modify | Add top-bar icon button + route to "modifiers" page |
| `src/lib/types.ts` | modify | Add `"modifiers"` to `Page` union; add `ModifierToggles`, `ModifierState`, `RawScancodePair` types |
| `src/lib/i18n.ts` | modify | Add ~15 new translation keys for the page |

## Edge cases

- **No Scancode Map value exists** → all toggles default to off; no warning. Normal initial state.
- **Scancode Map exists but is malformed** (rare; possible after a botched manual edit) → show error: "Format invalide. Cliquer Désactiver tout pour réinitialiser."
- **User applies, then uninstalls MagicWindows** → Scancode Map persists (it's a system-wide registry value, not owned by our app). The "uninstall layout" flow does NOT clear it. If the user wants their normal Windows behavior back, they use the "Désactiver tout" button on this page.
- **Per-side toggle + both-sides interaction in reverse-derivation** — if registry has only LCtrl↔LWin, "both sides" is off and "left only" is on. If both per-side pairs are present, prefer "both sides" (cleaner mental model).
- **Other extended-scancode mappings** (e.g. user has SharpKeys remapping Print Screen to nothing) → `has_external_mappings = true`. We warn but allow overwrite.
- **Apply with zero toggles checked + no existing value** → no-op, success toast: "Aucun changement nécessaire."

## Out of scope

- Custom user-defined remappings beyond the 5 toggles (SharpKeys-style full UI). Those tools already exist; we focus on Apple-on-Windows ergonomics.
- Per-application remappings (Karabiner-style, would require kernel driver).
- Live preview without reboot (PowerToys can do this; we'd need PT installed — adds dependency).
- Detecting exactly which external tool wrote conflicting entries (not technically possible from the Scancode Map alone — entries don't carry attribution).
- A "Reboot now" button that auto-reboots — too aggressive; we link to "Démarrer > Redémarrer" instead.

## Resolved decisions

1. **Toggle set:** the 5 listed above. ✅
2. **UI placement:** standalone page reached from a top-bar icon, with a built-in two-step Select → Preview → Apply flow. ✅
3. **Conflict handling:** warn and let the user decide; never block. ✅
