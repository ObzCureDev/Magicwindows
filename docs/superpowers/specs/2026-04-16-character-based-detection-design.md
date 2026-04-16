# Character-Based Keyboard Layout Detection

**Date:** 2026-04-16
**Status:** Design — pending review
**Replaces:** Position-based detection currently in [src/pages/Detect.svelte](../../../src/pages/Detect.svelte)

## Problem

The current detection flow asks the user to press keys at named physical positions ("press the first key on the upper letter row"). This is unintuitive — users identify keys by the **symbol printed on them**, not by abstract row/column descriptions. The visual mockup helps but the cognitive load is real.

## Solution

Ask the user to type a **character that is printed on their physical Apple keyboard** ("press @"). Capture `event.code` (the physical key position, layout-independent) and match it to the position where each Apple layout prints that character.

## The key insight

`event.code` always returns the physical key position regardless of the current Windows layout. So if the user looks at their physical keyboard, finds the key marked "@", and presses it (with whatever modifier the printed-symbol position requires), we get the position deterministically — without caring what character Windows actually produces.

## Algorithm

### Data: where is each character printed?

Derived from inspection of the six layouts in `layouts/*.json`:

**`@` (U+0040) physical position per Apple layout:**

| Layout | Scancode | event.code | Layer |
|--------|----------|------------|-------|
| apple-us-qwerty | 03 | Digit2 | shift |
| apple-uk-qwerty | 28 | Quote | shift |
| apple-fr-azerty | 29 | Backquote | base (top-left, very prominent) |
| apple-de-qwertz | 26 | KeyL | altgr (Option) |
| apple-es-qwerty | 03 | Digit2 | altgr |
| apple-it-qwerty | 27 | Semicolon | altgr |

**`ñ` (U+00F1) — exists only on ES at scancode 27 (Semicolon).**

(Layer doesn't matter for detection — we only care about `event.code`.)

### Iterative narrowing (pseudocode)

```
candidates = [all 6 layout ids]
attempts = 0
while len(candidates) > 1 and attempts < MAX_ATTEMPTS:
    char = pick_best_question(candidates)
    expected = { layout_id: physical_position_of(char, layout) for layout in candidates }
    response = await user_response()

    if response.kind == "no_such_key":
        # User says they don't see this character on their keyboard
        candidates = [c for c in candidates if char not in printed_chars(c)]
    elif response.kind == "key_pressed":
        if response.event_code in expected.values():
            candidates = [c for c in candidates if expected[c] == response.event_code]
            attempts += 1
        else:
            # Pressed something far off — show "oops" toast, no candidate change
            show_oops_toast()
    # ignore modifier-only keydowns

if len(candidates) == 1: return candidates[0]
else: fall_back_to_manual_selection()
```

### Best-question heuristic

Pick the character that produces the most balanced split of remaining candidates.
For the initial 6-layout set, asking `@` first is near-optimal:

- `Digit2` → {US, ES}
- `Quote` → {UK}
- `Backquote` → {FR}
- `KeyL` → {DE}
- `Semicolon` → {IT}

After `@`: 4 layouts are uniquely identified, only {US, ES} need a second question. Ask `ñ`:

- key pressed at `Semicolon` → ES
- "I don't have this key" → US

**Worst-case: 2 questions.** Best case: 1 question.

### "Oops, wrong key" handling

After every keydown:

| Event | Action |
|-------|--------|
| Modifier-only (`Shift`, `Alt`, `Control`, `Meta`, `Fn`) | Ignore — wait for the actual key |
| `event.code` matches an expected position for any candidate | Reduce candidates, advance |
| `event.code` is anywhere else (asked Q, pressed Z/W/S/X) | Show toast: *"Oups, ce n'est peut-être pas la bonne touche. Réessayez."* — keep candidates intact |
| `Escape` | Cancel detection, return to Welcome |

The toast auto-dismisses after ~2.5s. The user can then try again.

### "Je n'ai pas cette touche" button

Visible only when the asked character does **not** exist on every candidate layout (i.e., regional special characters like `ñ`, `£`, `ç`).

Click semantics: removes from candidates every layout that **does** have the character (because the user is saying their keyboard doesn't have it).

## UI / UX

### Detect.svelte layout

```
┌──────────────────────────────────────────────────┐
│  [progress bar — N/MAX questions]                │
│                                                  │
│  Appuyez sur la touche marquée                   │
│                                                  │
│              ┌─────┐                             │
│              │  @  │      ← large character card │
│              └─────┘                             │
│                                                  │
│  Le symbole peut être petit, en haut ou sur le   │
│  côté de la touche. Vous devrez peut-être        │
│  utiliser Maj ou Option.                         │
│                                                  │
│  [Je n'ai pas cette touche]   (conditional)      │
│                                                  │
│  [keyboard mockup — neutral, no highlight]       │
└──────────────────────────────────────────────────┘
```

When "oops" toast fires, it appears as a transient banner above the prompt:
> *Oups, ce n'est peut-être pas la bonne touche. Réessayez.*

### Why no highlight on the mockup?

The mockup stays neutral — highlighting the expected position would defeat the purpose (users would press based on our hint instead of finding the symbol on their physical keyboard, breaking the detection).

### i18n strings to add

| Key | EN | FR |
|-----|----|----|
| `detect.charPrompt` | Press the key marked | Appuyez sur la touche marquée |
| `detect.charHint` | The symbol may be small, on top, or on the side. You may need to use Shift or Option. | Le symbole peut être petit, en haut ou sur le côté de la touche. Vous devrez peut-être utiliser Maj ou Option. |
| `detect.noKey` | I don't have this key | Je n'ai pas cette touche |
| `detect.wrongKey` | Oops, that might be the wrong key. Try again. | Oups, ce n'est peut-être pas la bonne touche. Réessayez. |
| `detect.giveUp` | Couldn't detect — pick manually | Détection impossible — sélection manuelle |

## Architecture: where the logic lives

**Recommendation: pure frontend.** The character-position table is small (6 layouts × ~5 distinguishing chars), the matching logic is trivial JS, and avoiding Tauri round-trips makes the UX snappier.

The data can come from either:

- **(A)** A new Tauri command `get_detection_chars()` that reads layouts at startup and returns the table, OR
- **(B)** A static derived JSON shipped at build time via `src-tauri/build.rs`

**Preferred: (A)** — keeps a single source of truth in `layouts/*.json`. The build step approach risks getting out of sync.

The existing Tauri commands (`get_detection_keys`, `match_detection`) become unused for this flow. Decision: leave them in place, mark them deprecated; remove in a follow-up if no other consumer surfaces.

## Files to change

| File | Change |
|------|--------|
| `src-tauri/src/keyboard/mod.rs` (or `detect.rs`) | New command `get_detection_chars() -> Vec<DetectionChar>` returning `{ char: String, positions: HashMap<LayoutId, EventCode> }` |
| `src-tauri/src/keyboard/scancode.rs` (new or existing) | Helper: scancode (e.g. `"03"`) → DOM `event.code` (e.g. `"Digit2"`). Pure lookup table. |
| `src/lib/types.ts` | Add `DetectionChar`, `DetectionResponse`, `DetectionState` types |
| `src/pages/Detect.svelte` | Replace position-based flow with character-based flow + oops toast + no-key button |
| `src/lib/i18n.ts` | Add 5 keys above |

`KeyboardVisual.svelte` and the inline `mk-*` mockup styles are unchanged.

## Edge cases

- **Modifier-only keydown** — ignore, wait for the next non-modifier event
- **Dead key** (e.g., user presses `^` first) — treat as a normal keypress; check `event.code` of the dead key
- **Function/navigation keys** during prompt — treated as "wrong key" (oops toast)
- **User pastes via clipboard** — not supported; only keydown events count
- **MAX_ATTEMPTS exceeded** (suggest 5: at most 2 real questions plus 3 oops retries) — fall back to manual selection screen
- **Same physical position across layouts** — e.g. US and ES both have `@` at Digit2; resolved by the next question
- **User truly has no Apple keyboard** — fallback button to "Pick manually" remains visible at all times

## Out of scope

- Highlighting the expected position on the mockup (deliberately omitted, see UX section)
- Visual symbol overlay on the mockup keys
- Detecting custom Windows layouts the user may have installed
- Statistical learning from prior detections to refine question order
- Voice/accessibility cues beyond existing aria labels

## Open questions for review

1. **MAX_ATTEMPTS** — proposed 5 (2 valid + 3 oops). Acceptable, or higher/lower?
2. **Fallback after failure** — go directly to manual `Select.svelte`, or show a dedicated "we couldn't detect" screen first? Proposed: skip the extra screen, route directly to Select with an inline status banner.
3. **Should the keyboard mockup stay on screen** during the prompt, or be hidden until detection finishes? Proposed: keep it visible (neutral, no highlight) — gives the user something to glance at while finding the symbol.
4. **Tauri command vs static JSON** — confirm preference for the runtime command approach (single source of truth).
