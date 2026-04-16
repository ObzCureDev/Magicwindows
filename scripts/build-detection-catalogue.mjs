#!/usr/bin/env node
// Reads layouts/apple-*.json, extracts every (codepoint, scancode) pair from base/shift/altgr/altgrShift layers,
// translates scancode -> DOM event.code, filters out dead keys (entries ending with '@'),
// and emits a per-character map of layout -> position. Only includes characters that distinguish
// at least 2 layouts (or are present in 1 layout — useful as ABSENT-bucket discriminators).

import { readFileSync, writeFileSync, readdirSync } from "fs";
import { join, resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { SCANCODE_TO_CODE } from "./scancode-map.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LAYOUTS_DIR = resolve(__dirname, "..", "layouts");
const OUT_PATH   = resolve(__dirname, "..", "src", "lib", "detection-catalogue.generated.json");

// Curated whitelist of characters that are universally and unambiguously printed on
// physical keyboards. Excludes glyphs that vary by font/locale (guillemets «», smart
// quotes "", em/en dashes, bullet •, OEM oddities) and dead-key prone marks.
// Goal: when we ask the user "press the key where you see X", X must be visually
// unmistakable on their physical keycap.
const WHITELIST = new Set([
  // ASCII punctuation/symbols
  "@", "#", "$", "%", "&", "?", "!", "*", "+", "=",
  "(", ")", "[", "]", "{", "}",
  "<", ">", "/", "\\", "|",
  // Currency
  "£", "€", "¥",
  // French / Italian printed accents (bare letters with diacritic on the keycap)
  "é", "è", "à", "ù", "ò", "ì",
  // Iberian / Germanic printed letters
  "ç", "ñ", "ä", "ö", "ü", "ß",
]);

function loadLayouts() {
  return readdirSync(LAYOUTS_DIR)
    .filter(f => f.startsWith("apple-") && f.endsWith(".json"))
    .map(f => JSON.parse(readFileSync(join(LAYOUTS_DIR, f), "utf8")));
}

function isDeadKey(hex) {
  return typeof hex === "string" && hex.endsWith("@");
}

function normalizeHex(hex) {
  return isDeadKey(hex) ? hex.slice(0, -1) : hex;
}

// Apple-on-Windows hardware quirk: on ISO Apple keyboards (FR, DE, IT, ES, UK), the
// top-left "section key" reports scancode 56 (IntlBackslash) instead of the standard
// PC scancode 29 (Backquote). The bottom-row 102nd key (between LShift and Z) reports
// scancode 29 instead of 56. The two are effectively swapped vs a generic PC ISO board.
// We can't tell at detection time which keyboard the user has, so we accept BOTH
// positions for whatever character the JSON puts on either of these scancodes.
const ISO_SECTION_ALIASES = { "Backquote": "IntlBackslash", "IntlBackslash": "Backquote" };
function isISOLayout(layout) {
  return Boolean(layout.keys && layout.keys["56"]);
}

// Returns Map<codepointHex, eventCode[]> — every scancode that prints this codepoint
// on this layout, plus Apple-quirk aliases. Order is preserved (canonical position first).
function charsPrintedOn(layout) {
  const result = new Map();
  const iso = isISOLayout(layout);
  const push = (cp, code) => {
    const arr = result.get(cp) ?? [];
    if (!arr.includes(code)) arr.push(code);
    result.set(cp, arr);
  };
  for (const [scancode, mapping] of Object.entries(layout.keys ?? {})) {
    const eventCode = SCANCODE_TO_CODE[scancode];
    if (!eventCode) continue;
    for (const layer of ["base", "shift", "altgr", "altgrShift"]) {
      const raw = mapping[layer];
      if (!raw || raw === "-1") continue;
      if (isDeadKey(raw)) continue;
      const cp = normalizeHex(raw).toLowerCase();
      if (cp === "0000") continue;
      push(cp, eventCode);
      if (iso && ISO_SECTION_ALIASES[eventCode]) {
        push(cp, ISO_SECTION_ALIASES[eventCode]);
      }
    }
  }
  return result;
}

function build() {
  const layouts = loadLayouts();
  // codepoint -> { layoutId -> eventCode }
  const byChar = new Map();
  for (const layout of layouts) {
    const charMap = charsPrintedOn(layout);
    for (const [cp, eventCode] of charMap) {
      if (!byChar.has(cp)) byChar.set(cp, {});
      byChar.get(cp)[layout.id] = eventCode;
    }
  }

  // Keep only characters useful for discrimination:
  //   - present on at least one layout AND
  //   - either: distinguishes 2+ layouts by position, OR splits present-vs-absent across the full set
  const totalLayouts = layouts.length;
  const useful = [];
  for (const [cp, positions] of byChar) {
    const char = String.fromCodePoint(parseInt(cp, 16));
    if (!WHITELIST.has(char)) continue; // only ask the user about unambiguously-printed glyphs
    const presentCount = Object.keys(positions).length;
    // Distinct CANONICAL position per layout (first entry of each array) — used both for
    // "does this question split candidates" and for the minimax score below.
    const canonicalPositions = new Set(Object.values(positions).map((arr) => arr[0]));
    const splitsPositions = canonicalPositions.size >= 2;
    const splitsPresence  = presentCount > 0 && presentCount < totalLayouts;
    if (splitsPositions || splitsPresence) {
      useful.push({ char, codepoint: cp, positions });
    }
  }

  // Sort: best discriminators first. Score by max bucket size, bucketed on the
  // canonical (first) position per layout + an ABSENT bucket. Aliases don't affect score.
  useful.sort((a, b) => {
    const score = (entry) => {
      const buckets = new Map();
      buckets.set("ABSENT", totalLayouts - Object.keys(entry.positions).length);
      for (const arr of Object.values(entry.positions)) {
        const code = arr[0];
        buckets.set(code, (buckets.get(code) ?? 0) + 1);
      }
      return Math.max(...buckets.values()); // smaller worst-bucket = better
    };
    return score(a) - score(b);
  });

  const out = { generatedAt: new Date().toISOString(), characters: useful };
  writeFileSync(OUT_PATH, JSON.stringify(out, null, 2) + "\n", "utf8");
  console.log(`Detection catalogue: ${useful.length} characters across ${layouts.length} layouts -> ${OUT_PATH}`);
}

build();
