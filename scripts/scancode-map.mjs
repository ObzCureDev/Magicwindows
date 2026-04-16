// Maps Windows PS/2 scancodes (as used in layouts/*.json) to DOM KeyboardEvent.code values.
// Coverage: every scancode that appears in any layouts/*.json key mapping.
// Source: USB HID Usage Tables + W3C UI Events KeyboardEvent code spec.

export const SCANCODE_TO_CODE = {
  // Top row
  "29": "Backquote",
  "02": "Digit1", "03": "Digit2", "04": "Digit3", "05": "Digit4",
  "06": "Digit5", "07": "Digit6", "08": "Digit7", "09": "Digit8",
  "0a": "Digit9", "0b": "Digit0",
  "0c": "Minus", "0d": "Equal",

  // Letter row 1 (QWERTY)
  "10": "KeyQ", "11": "KeyW", "12": "KeyE", "13": "KeyR", "14": "KeyT",
  "15": "KeyY", "16": "KeyU", "17": "KeyI", "18": "KeyO", "19": "KeyP",
  "1a": "BracketLeft", "1b": "BracketRight",

  // Letter row 2
  "1e": "KeyA", "1f": "KeyS", "20": "KeyD", "21": "KeyF", "22": "KeyG",
  "23": "KeyH", "24": "KeyJ", "25": "KeyK", "26": "KeyL",
  "27": "Semicolon", "28": "Quote", "2b": "Backslash",

  // Letter row 3
  "56": "IntlBackslash",
  "2c": "KeyZ", "2d": "KeyX", "2e": "KeyC", "2f": "KeyV", "30": "KeyB",
  "31": "KeyN", "32": "KeyM",
  "33": "Comma", "34": "Period", "35": "Slash",

  // Space
  "39": "Space",
};
