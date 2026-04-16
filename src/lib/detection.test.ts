import { describe, it, expect } from "vitest";
import { layoutsWithChar } from "./detection";
import type { DetectionCharEntry } from "./types";

const AT: DetectionCharEntry = {
  char: "@",
  codepoint: "0040",
  positions: {
    "apple-us-qwerty": "Digit2",
    "apple-uk-qwerty": "Quote",
    "apple-fr-azerty": "Backquote",
    "apple-de-qwertz": "KeyL",
    "apple-es-qwerty": "Digit2",
    "apple-it-qwerty": "Semicolon",
  },
};

const NTILDE: DetectionCharEntry = {
  char: "ñ",
  codepoint: "00f1",
  positions: { "apple-es-qwerty": "Semicolon" },
};

describe("layoutsWithChar", () => {
  it("returns all candidates that have the char printed", () => {
    const result = layoutsWithChar(AT, ["apple-us-qwerty", "apple-fr-azerty"]);
    expect(result).toEqual(["apple-us-qwerty", "apple-fr-azerty"]);
  });

  it("excludes candidates that do not have the char", () => {
    const result = layoutsWithChar(NTILDE, ["apple-us-qwerty", "apple-es-qwerty"]);
    expect(result).toEqual(["apple-es-qwerty"]);
  });

  it("returns empty array when no candidate has the char", () => {
    const result = layoutsWithChar(NTILDE, ["apple-us-qwerty", "apple-fr-azerty"]);
    expect(result).toEqual([]);
  });
});
