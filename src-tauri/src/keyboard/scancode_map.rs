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

// Suppress dead-code warnings during multi-task implementation. Tasks 4-6 wire
// these symbols into the Tauri command layer; remove the attribute once done.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierToggles {
    pub swap_cmd_ctrl_left:  bool,
    pub swap_cmd_ctrl_right: bool,
    pub caps_to_ctrl:        bool,
    pub swap_option_cmd:     bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Reverse-derive which toggles the user has already enabled, based on the raw
/// pairs read from the registry. Pairs that don't match any known toggle group
/// flip `has_external_mappings = true` (used by the UI to warn before overwrite).
pub fn derive_state(pairs: &[RawScancodePair]) -> ModifierState {
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
