# Mac-Style Modifier Keys — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a separate page in MagicWindows that lets the user toggle Mac-style modifier remappings (Cmd↔Ctrl swap, Caps Lock→Ctrl, Option↔Cmd) by writing a binary `Scancode Map` value to `HKLM\System\CurrentControlSet\Control\Keyboard Layout`.

**Architecture:** A pure Rust serializer/parser handles the Microsoft Scancode Map binary format (8-byte header + N×4-byte mapping entries + null terminator). Three Tauri commands wrap this: `read_scancode_map` (un-elevated PowerShell read), `write_scancode_map` (elevated PS write), `clear_scancode_map` (elevated PS delete). A new `Modifiers.svelte` page provides Select → Preview → Apply UX. Reboot required for Windows to pick up the changes.

**Tech Stack:** Rust (cargo built-in tests for binary logic), PowerShell for registry I/O, Svelte 5 + TypeScript, Tauri v2.

**Spec:** [docs/superpowers/specs/2026-04-17-mac-style-modifiers-design.md](../specs/2026-04-17-mac-style-modifiers-design.md)

---

## File Structure

| File | Status | Responsibility |
|------|--------|----------------|
| `src-tauri/src/keyboard/scancode_map.rs` | NEW | Pure binary serialize/parse + canonical-pair detection (no I/O, fully unit-testable) |
| `src-tauri/src/keyboard/modifiers.rs` | NEW | 3 Tauri commands wrapping PowerShell I/O around the registry |
| `src-tauri/src/keyboard/mod.rs` | modify | `pub mod scancode_map; pub mod modifiers;` |
| `src-tauri/src/lib.rs` | modify | Register the 3 new commands in `invoke_handler!` |
| `src/lib/types.ts` | modify | Add `"modifiers"` to `Page`; add `ModifierToggles`, `ModifierState`, `RawScancodePair` |
| `src/lib/i18n.ts` | modify | ~15 new translation keys for the page (EN + FR) |
| `src/pages/Modifiers.svelte` | NEW | Two-step UI (toggle list → preview & apply) |
| `src/App.svelte` | modify | Top-bar icon button + `{:else if appState.page === "modifiers"}` route |

The Rust binary logic lives in its own file (`scancode_map.rs`) precisely so it can be unit-tested without any registry/PowerShell mocking. The thin command layer (`modifiers.rs`) does the I/O.

---

## Task 1: Frontend & backend types

**Files:**
- Modify: `src/lib/types.ts`
- Create: `src-tauri/src/keyboard/scancode_map.rs` (skeleton only)
- Modify: `src-tauri/src/keyboard/mod.rs`

- [ ] **Step 1: Add frontend types to `src/lib/types.ts`**

Append at the end:

```ts
// ── Mac-style modifier keys (see docs/superpowers/specs/2026-04-17-mac-style-modifiers-design.md)

export interface ModifierToggles {
  swapCmdCtrlLeft: boolean;
  swapCmdCtrlRight: boolean;
  capsToCtrl: boolean;
  swapOptionCmd: boolean;
}

export interface RawScancodePair {
  /** 4-hex-char little-endian scancode emitted (e.g. "1D00" for LCtrl). */
  newCode: string;
  /** 4-hex-char little-endian scancode received from the keyboard. */
  oldCode: string;
}

export interface ModifierState {
  /** Best-effort reverse-derivation of which toggles match the current registry value. */
  current: ModifierToggles;
  /** True if the registry has entries that don't correspond to any of our toggles. */
  hasExternalMappings: boolean;
  /** All raw pairs found in the registry (for the warning details). */
  rawEntries: RawScancodePair[];
}
```

Add `"modifiers"` to the existing `Page` union:

```ts
export type Page = "welcome" | "detect" | "select" | "preview" | "install" | "done" | "about" | "modifiers";
```

- [ ] **Step 2: Create `src-tauri/src/keyboard/scancode_map.rs` skeleton**

```rust
//! Microsoft Scancode Map binary serialization & parsing.
//!
//! The `HKLM\System\CurrentControlSet\Control\Keyboard Layout\Scancode Map`
//! registry value is a `REG_BINARY` whose layout is:
//!   - 8 bytes header (all zeros)
//!   - 4 bytes little-endian count = (number of mapping entries) + 1 for the terminator
//!   - N × 4 bytes mappings: 2 bytes "new" scancode + 2 bytes "old" scancode (little-endian)
//!   - 4 bytes null terminator
//!
//! A scancode is 2 bytes: low byte first, then high byte (00 = normal, E0 = extended).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierToggles {
    pub swap_cmd_ctrl_left:  bool,
    pub swap_cmd_ctrl_right: bool,
    pub caps_to_ctrl:        bool,
    pub swap_option_cmd:     bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawScancodePair {
    pub new_code: String, // 4 hex chars, little-endian (e.g. "1D00")
    pub old_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierState {
    pub current: ModifierToggles,
    pub has_external_mappings: bool,
    pub raw_entries: Vec<RawScancodePair>,
}

// Stubs — implementations land in tasks 2 and 3.
pub fn build_scancode_map(_toggles: &ModifierToggles) -> Vec<u8> {
    unimplemented!("Task 2")
}

pub fn parse_scancode_map(_bytes: &[u8]) -> Result<Vec<RawScancodePair>, String> {
    unimplemented!("Task 3")
}

pub fn derive_state(_pairs: &[RawScancodePair]) -> ModifierState {
    unimplemented!("Task 3")
}
```

- [ ] **Step 3: Re-export the new module from `src-tauri/src/keyboard/mod.rs`**

Add at the top, after the existing `pub mod` lines:

```rust
pub mod scancode_map;
pub mod modifiers;
```

(`modifiers` doesn't exist yet — that's Task 4. Add the line now anyway and silence the resulting compile error in the next sub-step by also adding a `modifiers.rs` placeholder.)

Create `src-tauri/src/keyboard/modifiers.rs` placeholder (just enough to compile):

```rust
//! Tauri command layer for the Scancode Map registry value.
//! Real commands land in tasks 4-6.
```

- [ ] **Step 4: Verify everything still compiles**

Run from the repo root:

```bash
cd src-tauri && cargo check 2>&1 | tail -5 && cd ..
npm run check 2>&1 | tail -3
```

Expected: cargo finishes with `Finished dev profile`; svelte-check reports `0 ERRORS 0 WARNINGS`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src-tauri/src/keyboard/scancode_map.rs src-tauri/src/keyboard/modifiers.rs src-tauri/src/keyboard/mod.rs
git commit -m "feat: types and scaffolding for mac-style modifier feature"
```

---

## Task 2: Implement `build_scancode_map` (TDD)

**Files:**
- Modify: `src-tauri/src/keyboard/scancode_map.rs`

- [ ] **Step 1: Add the failing tests at the bottom of `scancode_map.rs`**

```rust
#[cfg(test)]
mod build_tests {
    use super::*;

    fn header(entry_count: u32) -> Vec<u8> {
        let mut v = vec![0u8; 8];
        v.extend_from_slice(&entry_count.to_le_bytes());
        v
    }

    fn terminator() -> [u8; 4] { [0, 0, 0, 0] }

    #[test]
    fn empty_toggles_produces_empty_vec() {
        let bytes = build_scancode_map(&ModifierToggles::default());
        assert!(bytes.is_empty(),
            "all-off toggles should produce an empty vec so the caller can DELETE the registry value");
    }

    #[test]
    fn caps_to_ctrl_only() {
        let bytes = build_scancode_map(&ModifierToggles { caps_to_ctrl: true, ..Default::default() });
        let mut expected = header(2); // 1 mapping + 1 terminator
        expected.extend_from_slice(&[0x1D, 0x00, 0x3A, 0x00]); // LCtrl ← CapsLock
        expected.extend_from_slice(&terminator());
        assert_eq!(bytes, expected);
    }

    #[test]
    fn swap_cmd_ctrl_left_produces_two_entries() {
        let bytes = build_scancode_map(&ModifierToggles { swap_cmd_ctrl_left: true, ..Default::default() });
        let mut expected = header(3); // 2 mappings + 1 terminator
        expected.extend_from_slice(&[0x1D, 0x00, 0x5B, 0xE0]); // LCtrl ← LWin
        expected.extend_from_slice(&[0x5B, 0xE0, 0x1D, 0x00]); // LWin ← LCtrl
        expected.extend_from_slice(&terminator());
        assert_eq!(bytes, expected);
    }

    #[test]
    fn swap_cmd_ctrl_both_sides() {
        let bytes = build_scancode_map(&ModifierToggles {
            swap_cmd_ctrl_left:  true,
            swap_cmd_ctrl_right: true,
            ..Default::default()
        });
        let mut expected = header(5); // 4 mappings + 1 terminator
        expected.extend_from_slice(&[0x1D, 0x00, 0x5B, 0xE0]); // LCtrl ← LWin
        expected.extend_from_slice(&[0x5B, 0xE0, 0x1D, 0x00]); // LWin ← LCtrl
        expected.extend_from_slice(&[0x1D, 0xE0, 0x5C, 0xE0]); // RCtrl ← RWin
        expected.extend_from_slice(&[0x5C, 0xE0, 0x1D, 0xE0]); // RWin ← RCtrl
        expected.extend_from_slice(&terminator());
        assert_eq!(bytes, expected);
    }

    #[test]
    fn swap_option_cmd_both_sides() {
        let bytes = build_scancode_map(&ModifierToggles { swap_option_cmd: true, ..Default::default() });
        let mut expected = header(5);
        expected.extend_from_slice(&[0x38, 0x00, 0x5B, 0xE0]); // LAlt ← LWin
        expected.extend_from_slice(&[0x5B, 0xE0, 0x38, 0x00]); // LWin ← LAlt
        expected.extend_from_slice(&[0x38, 0xE0, 0x5C, 0xE0]); // RAlt ← RWin
        expected.extend_from_slice(&[0x5C, 0xE0, 0x38, 0xE0]); // RWin ← RAlt
        expected.extend_from_slice(&terminator());
        assert_eq!(bytes, expected);
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
cd src-tauri && cargo test --lib scancode_map::build_tests 2>&1 | tail -10
```

Expected: 5 tests fail with `not yet implemented` (from the `unimplemented!()` stub).

- [ ] **Step 3: Implement `build_scancode_map`**

Replace the `build_scancode_map` stub in `src-tauri/src/keyboard/scancode_map.rs`:

```rust
// ── Scancode constants (low byte, high byte) — high byte 0xE0 marks "extended" ───
const LCTRL: [u8; 2] = [0x1D, 0x00];
const RCTRL: [u8; 2] = [0x1D, 0xE0];
const LWIN:  [u8; 2] = [0x5B, 0xE0];
const RWIN:  [u8; 2] = [0x5C, 0xE0];
const LALT:  [u8; 2] = [0x38, 0x00];
const RALT:  [u8; 2] = [0x38, 0xE0];
const CAPS:  [u8; 2] = [0x3A, 0x00];

/// Encode a single mapping: pressing `old` causes the OS to report `new`.
fn entry(new: [u8; 2], old: [u8; 2]) -> [u8; 4] {
    [new[0], new[1], old[0], old[1]]
}

/// Build the binary Scancode Map value from the user's toggle selection.
/// Returns an EMPTY vec when no toggles are active — the caller should DELETE
/// the registry value rather than write a header-only blob.
pub fn build_scancode_map(toggles: &ModifierToggles) -> Vec<u8> {
    let mut entries: Vec<[u8; 4]> = Vec::new();

    if toggles.swap_cmd_ctrl_left {
        entries.push(entry(LCTRL, LWIN));   // LWin pressed → LCtrl emitted
        entries.push(entry(LWIN,  LCTRL));  // LCtrl pressed → LWin emitted
    }
    if toggles.swap_cmd_ctrl_right {
        entries.push(entry(RCTRL, RWIN));
        entries.push(entry(RWIN,  RCTRL));
    }
    if toggles.caps_to_ctrl {
        entries.push(entry(LCTRL, CAPS));   // one-way: CapsLock → LCtrl
    }
    if toggles.swap_option_cmd {
        entries.push(entry(LALT, LWIN));
        entries.push(entry(LWIN, LALT));
        entries.push(entry(RALT, RWIN));
        entries.push(entry(RWIN, RALT));
    }

    if entries.is_empty() {
        return Vec::new();
    }

    let count = (entries.len() + 1) as u32; // +1 for the null terminator
    let mut buf = Vec::with_capacity(8 + 4 + entries.len() * 4 + 4);
    buf.extend_from_slice(&[0u8; 8]);                  // header
    buf.extend_from_slice(&count.to_le_bytes());       // entry count
    for e in &entries {
        buf.extend_from_slice(e);
    }
    buf.extend_from_slice(&[0u8; 4]);                  // null terminator
    buf
}
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
cd src-tauri && cargo test --lib scancode_map::build_tests 2>&1 | tail -5
```

Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/keyboard/scancode_map.rs
git commit -m "feat(modifiers): build_scancode_map serializer + tests"
```

---

## Task 3: Implement `parse_scancode_map` and `derive_state` (TDD)

**Files:**
- Modify: `src-tauri/src/keyboard/scancode_map.rs`

- [ ] **Step 1: Add the failing tests below the existing test module**

```rust
#[cfg(test)]
mod parse_tests {
    use super::*;

    fn make_pair(new_lo: u8, new_hi: u8, old_lo: u8, old_hi: u8) -> RawScancodePair {
        RawScancodePair {
            new_code: format!("{:02X}{:02X}", new_lo, new_hi),
            old_code: format!("{:02X}{:02X}", old_lo, old_hi),
        }
    }

    #[test]
    fn parse_empty_input_returns_empty_list() {
        assert_eq!(parse_scancode_map(&[]).unwrap(), Vec::new());
    }

    #[test]
    fn parse_header_only_blob() {
        // 8-byte header + count=1 (just terminator) + 4-byte terminator = 16 bytes
        let bytes = [0u8; 16];
        let mut bytes_with_count = bytes.to_vec();
        bytes_with_count[8] = 1; // count = 1
        assert_eq!(parse_scancode_map(&bytes_with_count).unwrap(), Vec::new());
    }

    #[test]
    fn parse_caps_to_ctrl_blob() {
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&2u32.to_le_bytes());      // count = 2
        bytes.extend_from_slice(&[0x1D, 0x00, 0x3A, 0x00]); // LCtrl ← CapsLock
        bytes.extend_from_slice(&[0u8; 4]);                 // terminator
        let parsed = parse_scancode_map(&bytes).unwrap();
        assert_eq!(parsed, vec![make_pair(0x1D, 0x00, 0x3A, 0x00)]);
    }

    #[test]
    fn parse_rejects_truncated_input() {
        // Header says 5 entries but only 1 follows
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(&[0x1D, 0x00, 0x3A, 0x00]);
        // missing 3 entries + terminator
        assert!(parse_scancode_map(&bytes).is_err());
    }

    #[test]
    fn derive_state_recognizes_caps_only() {
        let pairs = vec![make_pair(0x1D, 0x00, 0x3A, 0x00)];
        let state = derive_state(&pairs);
        assert!(state.current.caps_to_ctrl);
        assert!(!state.current.swap_cmd_ctrl_left);
        assert!(!state.has_external_mappings);
    }

    #[test]
    fn derive_state_recognizes_cmd_ctrl_left() {
        let pairs = vec![
            make_pair(0x1D, 0x00, 0x5B, 0xE0), // LCtrl ← LWin
            make_pair(0x5B, 0xE0, 0x1D, 0x00), // LWin ← LCtrl
        ];
        let state = derive_state(&pairs);
        assert!(state.current.swap_cmd_ctrl_left);
        assert!(!state.current.swap_cmd_ctrl_right);
        assert!(!state.has_external_mappings);
    }

    #[test]
    fn derive_state_flags_external_mappings() {
        // Half a swap pair (only one direction) — not a recognized toggle
        let pairs = vec![make_pair(0x1D, 0x00, 0x5B, 0xE0)];
        let state = derive_state(&pairs);
        assert!(!state.current.swap_cmd_ctrl_left);
        assert!(state.has_external_mappings);
    }

    #[test]
    fn derive_state_recognizes_compound_toggles() {
        // Cmd↔Ctrl both sides + Caps→Ctrl
        let pairs = vec![
            make_pair(0x1D, 0x00, 0x5B, 0xE0),
            make_pair(0x5B, 0xE0, 0x1D, 0x00),
            make_pair(0x1D, 0xE0, 0x5C, 0xE0),
            make_pair(0x5C, 0xE0, 0x1D, 0xE0),
            make_pair(0x1D, 0x00, 0x3A, 0x00),
        ];
        let state = derive_state(&pairs);
        assert!(state.current.swap_cmd_ctrl_left);
        assert!(state.current.swap_cmd_ctrl_right);
        assert!(state.current.caps_to_ctrl);
        assert!(!state.current.swap_option_cmd);
        assert!(!state.has_external_mappings);
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
cd src-tauri && cargo test --lib scancode_map::parse_tests 2>&1 | tail -10
```

Expected: 8 tests fail with `not yet implemented`.

- [ ] **Step 3: Implement `parse_scancode_map`**

Replace the `parse_scancode_map` stub:

```rust
/// Parse a raw Scancode Map binary into a list of (new, old) pairs.
/// Returns an empty list for empty input or a header-only blob.
pub fn parse_scancode_map(bytes: &[u8]) -> Result<Vec<RawScancodePair>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 16 {
        return Err(format!("Scancode Map too short: {} bytes (need >= 16)", bytes.len()));
    }
    // Skip 8-byte header, read count (LE u32) at offset 8
    let count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if count == 0 {
        return Err("Scancode Map count is 0; expected >= 1 (terminator)".to_string());
    }
    // Body = count * 4 bytes (the last 4 bytes are the terminator)
    let body_len = count * 4;
    let expected_total = 12 + body_len;
    if bytes.len() < expected_total {
        return Err(format!(
            "Scancode Map truncated: header says {} entries (need {} bytes total) but got {}",
            count, expected_total, bytes.len()
        ));
    }
    let mut pairs = Vec::with_capacity(count.saturating_sub(1));
    for i in 0..count {
        let off = 12 + i * 4;
        let chunk = &bytes[off..off + 4];
        // Last entry should be the terminator (all zeros) — skip it
        if i == count - 1 {
            if chunk != [0u8; 4] {
                return Err("Scancode Map missing null terminator".to_string());
            }
            break;
        }
        pairs.push(RawScancodePair {
            new_code: format!("{:02X}{:02X}", chunk[0], chunk[1]),
            old_code: format!("{:02X}{:02X}", chunk[2], chunk[3]),
        });
    }
    Ok(pairs)
}
```

- [ ] **Step 4: Implement `derive_state`**

Replace the `derive_state` stub:

```rust
/// Reverse-derive which toggles the user has already enabled, based on the raw
/// pairs read from the registry. Pairs that don't match any known toggle group
/// flip `has_external_mappings = true` (used by the UI to warn before overwrite).
pub fn derive_state(pairs: &[RawScancodePair]) -> ModifierState {
    // Build a lookup set of all known pairs we would emit, grouped by toggle.
    fn pair(new: [u8; 2], old: [u8; 2]) -> RawScancodePair {
        RawScancodePair {
            new_code: format!("{:02X}{:02X}", new[0], new[1]),
            old_code: format!("{:02X}{:02X}", old[0], old[1]),
        }
    }

    let cmd_ctrl_left  = vec![pair(LCTRL, LWIN), pair(LWIN, LCTRL)];
    let cmd_ctrl_right = vec![pair(RCTRL, RWIN), pair(RWIN, RCTRL)];
    let caps           = vec![pair(LCTRL, CAPS)];
    let option_cmd     = vec![
        pair(LALT, LWIN), pair(LWIN, LALT),
        pair(RALT, RWIN), pair(RWIN, RALT),
    ];

    let pair_set: std::collections::HashSet<&RawScancodePair> = pairs.iter().collect();
    let group_present = |group: &[RawScancodePair]| group.iter().all(|p| pair_set.contains(p));

    let toggles = ModifierToggles {
        swap_cmd_ctrl_left:  group_present(&cmd_ctrl_left),
        swap_cmd_ctrl_right: group_present(&cmd_ctrl_right),
        caps_to_ctrl:        group_present(&caps),
        swap_option_cmd:     group_present(&option_cmd),
    };

    // External = any pair in the registry that isn't part of an active group.
    let mut accounted: std::collections::HashSet<&RawScancodePair> = std::collections::HashSet::new();
    if toggles.swap_cmd_ctrl_left  { for p in &cmd_ctrl_left  { accounted.insert(p); } }
    if toggles.swap_cmd_ctrl_right { for p in &cmd_ctrl_right { accounted.insert(p); } }
    if toggles.caps_to_ctrl        { for p in &caps           { accounted.insert(p); } }
    if toggles.swap_option_cmd     { for p in &option_cmd     { accounted.insert(p); } }

    let has_external_mappings = pairs.iter().any(|p| !accounted.contains(p));

    ModifierState {
        current: toggles,
        has_external_mappings,
        raw_entries: pairs.to_vec(),
    }
}
```

- [ ] **Step 5: Run tests, verify all pass**

```bash
cd src-tauri && cargo test --lib scancode_map 2>&1 | tail -8
```

Expected: `test result: ok. 13 passed` (5 from Task 2 + 8 from Task 3).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/keyboard/scancode_map.rs
git commit -m "feat(modifiers): parse_scancode_map + derive_state with reverse-derivation"
```

---

## Task 4: Tauri command `read_scancode_map`

**Files:**
- Modify: `src-tauri/src/keyboard/modifiers.rs`

- [ ] **Step 1: Replace `src-tauri/src/keyboard/modifiers.rs` with the read command**

```rust
//! Tauri command layer for the Scancode Map registry value.
//!
//! Reading is unprivileged (HKLM is world-readable). Writing requires elevation
//! and goes through the same elevated-PowerShell helper used by the layout
//! installer in install.rs.

use super::scancode_map::{
    derive_state, parse_scancode_map, ModifierState, ModifierToggles,
};

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn read_scancode_map() -> Result<ModifierState, String> {
    use std::process::Command;

    // Read the REG_BINARY value as base64 so it survives the PowerShell -> stdout pipe cleanly.
    let script = r#"
$bytes = (Get-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Keyboard Layout' `
                            -Name 'Scancode Map' -ErrorAction SilentlyContinue).'Scancode Map'
if ($bytes) {
    [Convert]::ToBase64String($bytes)
} else {
    'NONE'
}
"#;

    let output = Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to invoke powershell: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "powershell read failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes: Vec<u8> = if stdout == "NONE" || stdout.is_empty() {
        Vec::new()
    } else {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(&stdout)
            .map_err(|e| format!("Bad base64 from PowerShell: {e}"))?
    };

    let pairs = parse_scancode_map(&bytes)?;
    Ok(derive_state(&pairs))
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn read_scancode_map() -> Result<ModifierState, String> {
    Err("Modifier remapping requires Windows.".to_string())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn write_scancode_map(_toggles: ModifierToggles) -> Result<(), String> {
    Err("Modifier remapping requires Windows.".to_string())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn clear_scancode_map() -> Result<(), String> {
    Err("Modifier remapping requires Windows.".to_string())
}
```

- [ ] **Step 2: Add the `base64` dependency to `src-tauri/Cargo.toml`**

Find the `[dependencies]` section and add:

```toml
base64 = "0.22"
```

- [ ] **Step 3: Verify it compiles**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: `Finished dev profile`. If `base64` fails to fetch, double-check the version pin.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/keyboard/modifiers.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(modifiers): read_scancode_map tauri command"
```

---

## Task 5: Tauri command `write_scancode_map` (elevated)

**Files:**
- Modify: `src-tauri/src/keyboard/modifiers.rs`

- [ ] **Step 1: Add the write command before the `#[cfg(not(target_os = ...))]` stubs**

```rust
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn write_scancode_map(toggles: ModifierToggles) -> Result<(), String> {
    use super::scancode_map::build_scancode_map;
    use crate::keyboard::install::get_install_dir;
    use std::fs;

    let bytes = build_scancode_map(&toggles);
    let install_dir = get_install_dir();
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create install dir: {e}"))?;

    // Empty bytes = user wants no mappings. Delegate to clear_scancode_map.
    if bytes.is_empty() {
        return delete_scancode_map_value(&install_dir);
    }

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {{
    throw "Administrator privileges are required to modify the keyboard layout registry."
}}

$path  = 'HKLM:\System\CurrentControlSet\Control\Keyboard Layout'
$name  = 'Scancode Map'
$bytes = [Convert]::FromBase64String('{b64}')

# Ensure the key exists (it does on every Windows install, but defensive).
if (-not (Test-Path -LiteralPath $path)) {{
    throw "Registry path not found: $path"
}}

Set-ItemProperty -LiteralPath $path -Name $name -Value $bytes -Type Binary -Force
Write-Host "Scancode Map written ($($bytes.Length) bytes)."
"#
    );

    super::install::run_elevated_ps_for_modifiers(&install_dir, "scancode_write", &script)?;
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn clear_scancode_map() -> Result<(), String> {
    use crate::keyboard::install::get_install_dir;
    use std::fs;
    let install_dir = get_install_dir();
    fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create install dir: {e}"))?;
    delete_scancode_map_value(&install_dir)
}

#[cfg(target_os = "windows")]
fn delete_scancode_map_value(install_dir: &std::path::Path) -> Result<(), String> {
    let script = r#"
$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Administrator privileges are required to modify the keyboard layout registry."
}

$path = 'HKLM:\System\CurrentControlSet\Control\Keyboard Layout'
Remove-ItemProperty -LiteralPath $path -Name 'Scancode Map' -ErrorAction SilentlyContinue
Write-Host "Scancode Map cleared (or already absent)."
"#;
    super::install::run_elevated_ps_for_modifiers(install_dir, "scancode_clear", script)?;
    Ok(())
}
```

- [ ] **Step 2: Expose the elevated PS helper from `install.rs`**

The existing `run_elevated_ps` function in `src-tauri/src/keyboard/install.rs` is private. Add a thin public wrapper at the end of the file (after the existing functions):

```rust
/// Public re-export of the elevated PowerShell runner so other modules in the
/// keyboard crate (e.g. modifiers.rs) can use the same UAC + capture logic.
#[cfg(target_os = "windows")]
pub fn run_elevated_ps_for_modifiers(
    work_dir: &std::path::Path,
    label: &str,
    ps_script: &str,
) -> Result<String, String> {
    run_elevated_ps(work_dir, label, ps_script)
}
```

(`run_elevated_ps` already returns `Result<String, String>` after the install-auto-activate change. The re-export keeps the original private to discourage external misuse.)

- [ ] **Step 3: Verify it compiles**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: `Finished dev profile`. No warnings.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/keyboard/modifiers.rs src-tauri/src/keyboard/install.rs
git commit -m "feat(modifiers): write_scancode_map and clear_scancode_map (elevated)"
```

---

## Task 6: Register the 3 commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the imports**

Find the existing `use crate::keyboard;` (or similar) at the top of `src-tauri/src/lib.rs`. After it, the existing `keyboard::install::install_layout` calls work because of the re-export. The new commands live in `keyboard::modifiers`.

In the `invoke_handler!` macro call (around line 155), add the three new commands. The block currently looks like:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    install_layout,
    uninstall_layout,
])
```

The new commands aren't `#[tauri::command]` defined in lib.rs — they're already decorated in `modifiers.rs`. We just need to reference them by their fully-qualified path:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    install_layout,
    uninstall_layout,
    crate::keyboard::modifiers::read_scancode_map,
    crate::keyboard::modifiers::write_scancode_map,
    crate::keyboard::modifiers::clear_scancode_map,
])
```

- [ ] **Step 2: Verify it compiles**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(modifiers): register the 3 new tauri commands"
```

---

## Task 7: i18n strings

**Files:**
- Modify: `src/lib/i18n.ts`

- [ ] **Step 1: Add a `modifiers` section to the `en` block**

Find the `en` top-level block in `src/lib/i18n.ts`. Add a new `modifiers` sub-section right before the `about` section (or anywhere inside `en`):

```ts
modifiers: {
  title: "Mac-style modifier keys",
  description:
    "Remap Cmd, Ctrl, Caps Lock, and Option to match Mac muscle memory. Changes are system-wide and require a reboot.",
  externalWarning:
    "External keyboard remappings detected (likely from SharpKeys, PowerToys, or similar). Applying your changes here will overwrite them.",
  externalDetails: "Show details",
  toggleSwapBoth: "Swap Cmd ↔ Ctrl (both sides)",
  toggleSwapBothHint: "Cmd+C copies, Cmd+V pastes, etc. — like macOS.",
  toggleSwapLeft: "Swap Cmd ↔ Ctrl (left only)",
  toggleSwapRight: "Swap Cmd ↔ Ctrl (right only)",
  toggleCaps: "Caps Lock → Ctrl",
  toggleCapsHint: "Adds an extra Ctrl under your left pinky — handy for Vim/Emacs.",
  toggleOptionCmd: "Swap Option ↔ Cmd (Mac-strict positions)",
  toggleOptionCmdHint:
    "Mutually exclusive with the Cmd ↔ Ctrl swaps — pick one or the other.",
  preview: "Preview changes",
  disableAll: "Disable all",
  back: "Back",
  previewTitle: "Review changes",
  previewBefore: "Before",
  previewAfter: "After",
  previewNoChange: "No changes selected.",
  rebootWarning: "A reboot is required for Windows to pick up the new mappings.",
  apply: "Apply now",
  applying: "Writing to registry…",
  cancel: "Cancel",
  applied: "Done. Reboot when convenient.",
  errorApply: "Failed to apply: {message}",
  topbarTitle: "Modifier keys",
},
```

- [ ] **Step 2: Mirror in `fr`**

In the `fr` top-level block, add the same section with French translations:

```ts
modifiers: {
  title: "Touches modificateurs Mac",
  description:
    "Remappez Cmd, Ctrl, Verr Maj et Option pour retrouver vos réflexes Mac. Les changements s'appliquent à tout le système et nécessitent un redémarrage.",
  externalWarning:
    "Des remappages clavier externes ont été détectés (SharpKeys, PowerToys, ou similaires). Les vôtres seront remplacés si vous appliquez ici.",
  externalDetails: "Voir les détails",
  toggleSwapBoth: "Échanger Cmd ↔ Ctrl (les deux côtés)",
  toggleSwapBothHint: "Cmd+C copie, Cmd+V colle, etc. — comme sur macOS.",
  toggleSwapLeft: "Échanger Cmd ↔ Ctrl (gauche seulement)",
  toggleSwapRight: "Échanger Cmd ↔ Ctrl (droite seulement)",
  toggleCaps: "Verr Maj → Ctrl",
  toggleCapsHint: "Ajoute un Ctrl supplémentaire sous l'auriculaire gauche — pratique pour Vim/Emacs.",
  toggleOptionCmd: "Échanger Option ↔ Cmd (positions Mac strict)",
  toggleOptionCmdHint:
    "Mutuellement exclusif avec les échanges Cmd ↔ Ctrl — choisissez l'un ou l'autre.",
  preview: "Aperçu des changements",
  disableAll: "Tout désactiver",
  back: "Retour",
  previewTitle: "Vérifier les changements",
  previewBefore: "Avant",
  previewAfter: "Après",
  previewNoChange: "Aucun changement sélectionné.",
  rebootWarning: "Un redémarrage est requis pour que Windows prenne en compte les nouveaux mappages.",
  apply: "Appliquer maintenant",
  applying: "Écriture dans le registre…",
  cancel: "Annuler",
  applied: "Terminé. Redémarrez quand vous voudrez.",
  errorApply: "Échec : {message}",
  topbarTitle: "Touches modificateurs",
},
```

- [ ] **Step 3: Verify TypeScript still compiles**

Run: `npm run check 2>&1 | tail -3`
Expected: `0 ERRORS 0 WARNINGS`.

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n.ts
git commit -m "feat(modifiers): i18n strings (EN + FR) for the new page"
```

---

## Task 8: Build `src/pages/Modifiers.svelte`

**Files:**
- Create: `src/pages/Modifiers.svelte`

- [ ] **Step 1: Create the file**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { appState } from "../lib/stores.svelte";
  import { t } from "../lib/i18n";
  import type { ModifierState, ModifierToggles, RawScancodePair } from "../lib/types";

  let state = $state<ModifierState | null>(null);
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
      state = s;
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
    return state?.rawEntries ?? [];
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
      {#if state?.hasExternalMappings}
        <div class="status status--warning" role="alert">
          {t(appState.lang, "modifiers.externalWarning")}
          <button class="link" onclick={() => (showExternalDetails = !showExternalDetails)}>
            {t(appState.lang, "modifiers.externalDetails")}
          </button>
          {#if showExternalDetails && state}
            <ul class="raw-pairs">
              {#each state.rawEntries as p}
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
```

- [ ] **Step 2: Verify TypeScript still compiles**

Run: `npm run check 2>&1 | tail -3`
Expected: `0 ERRORS 0 WARNINGS`. If errors mention the `<script context="module">` block, that's a Svelte 5 quirk — the helper can move into the main script block and be exported via a regular function instead. Move it inline if needed and re-run the check.

- [ ] **Step 3: Commit**

```bash
git add src/pages/Modifiers.svelte
git commit -m "feat(modifiers): Modifiers.svelte page with select + preview UX"
```

---

## Task 9: Wire the page into App.svelte

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Add the import at the top of the script block**

In `src/App.svelte` near the other page imports (Welcome, Detect, etc.), add:

```ts
import Modifiers from "./pages/Modifiers.svelte";
```

- [ ] **Step 2: Add the top-bar icon button**

In the `<div class="top-bar__controls">` block, add a button BEFORE the existing `theme-toggle`:

```svelte
<button
  class="theme-toggle"
  onclick={() => (appState.page = "modifiers")}
  title={t(appState.lang, "modifiers.topbarTitle")}
  aria-label={t(appState.lang, "modifiers.topbarTitle")}
>
  ⌘
</button>
```

(Reuses the existing `theme-toggle` class for matching size/style. The `⌘` glyph is universally recognized as the Cmd key.)

- [ ] **Step 3: Add the route case**

In the page-routing block, add a new `{:else if}` arm AFTER the `about` case:

```svelte
{:else if appState.page === "about"}
  <About />
{:else if appState.page === "modifiers"}
  <Modifiers />
{/if}
```

- [ ] **Step 4: Verify TypeScript still compiles**

Run: `npm run check 2>&1 | tail -3`
Expected: `0 ERRORS 0 WARNINGS`.

- [ ] **Step 5: Commit**

```bash
git add src/App.svelte
git commit -m "feat(modifiers): top-bar icon and route in App.svelte"
```

---

## Task 10: Final integration verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full Rust test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5 && cd ..
```

Expected: `test result: ok. 13 passed` (5 from build_tests + 8 from parse_tests).

- [ ] **Step 2: Run the full frontend test + check**

```bash
npm test 2>&1 | tail -5
npm run check 2>&1 | tail -3
```

Expected: 13 vitest tests passing (existing detection tests, untouched), 0 svelte-check errors.

- [ ] **Step 3: Build**

```bash
npm run build 2>&1 | tail -5
```

Expected: Vite build completes cleanly.

```bash
cd src-tauri && cargo build 2>&1 | tail -5 && cd ..
```

Expected: `Finished dev profile`.

- [ ] **Step 4: End-to-end smoke test in the dev app**

Run: `npm run tauri dev`

Walk through these flows:

| Scenario | Expected outcome |
|----------|------------------|
| Click ⌘ icon in top bar | Routes to Modifiers page; "Aucun" or current toggles shown |
| Check "Both sides" Cmd↔Ctrl, click Preview | Preview panel shows 4 mapping rows in "Après", reboot warning visible |
| Click Apply | UAC prompt → success toast → page refreshes showing Cmd↔Ctrl active |
| Re-enter the page | "Both sides" pre-checked from the registry read |
| Check Option↔Cmd | Cmd↔Ctrl checkboxes auto-disable; Option↔Cmd toggle becomes active |
| Click Disable All | UAC prompt → registry value cleared → toggles all unchecked |
| With a 3rd-party Scancode Map (e.g. SharpKeys) present | Yellow warning banner appears at top of select step |

- [ ] **Step 5: Commit any incidental cleanup; otherwise done**

```bash
git status
```

If clean, no commit needed. Otherwise commit with a descriptive message.

---

## Summary

After completing all tasks:

- Pure Rust scancode-map serializer/parser (`scancode_map.rs`) with 13 cargo tests covering normal cases and truncation/external-mapping detection.
- Three Tauri commands (`read_scancode_map`, `write_scancode_map`, `clear_scancode_map`) wrapping PowerShell I/O — read is unprivileged, write/clear are UAC-elevated through the same helper used by the layout installer.
- New `Modifiers.svelte` page with Select → Preview → Apply UX, mutual-exclusion logic between the Cmd↔Ctrl swaps and the Option↔Cmd swap, external-mapping warning, and a Disable All button.
- Top-bar `⌘` icon button to reach the page from anywhere.
