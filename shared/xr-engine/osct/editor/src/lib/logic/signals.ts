import type { PortKind, PortType, Signal } from "./types";

/**
 * **The signal grammar of the Logic canvas** (#334 L-T8): the SCUTOONS deck's `Signal` (deck-kit `base/signal.rs`) as one
 * table – each signal's pin shape, wire weight and colour by day and by night. The Logic canvas's pins, wires and box
 * heads draw from it and from nothing else (RULES #38's exception for the Logic canvas, Tilman 2026-11-21). The colours
 * are theme tokens (`--signal-<name>` in `app.css`, day and night; a test keeps them equal to this table).
 *
 * OSCT's own signals (`types.ts` `Signal` plus a port's type and kind) map to the closest deck signal with `deckSignal` –
 * the mapping is documented in agents/DESIGN.md ("Logic canvas: signals").
 */

/** The deck's signals OSCT draws (deck-kit `Signal`, same names and order – without the deck's `clock`: OSCT has no clock) */
export const DECK_SIGNALS = ["event", "control", "audio", "picture", "address", "vote", "scene", "far"] as const;
export type DeckSignal = (typeof DECK_SIGNALS)[number];

/** A pin's outline (deck-kit `Shape`): an event's triangle points along the flow, far is a hollow dot */
export type PinShape = "triangle" | "dot" | "square" | "diamond" | "ring";

export interface SignalStyle {
  shape: PinShape;
  /** A wire's width against the others: an event's is heavier – it is what makes things happen */
  weight: number;
  /** `#rrggbb` on the day panels and on the night panels */
  day: string;
  night: string;
}

/** The one table (deck-kit `Signal::shape`, `Signal::weight`, `Signals::day`, `Signals::night`) */
export const SIGNAL_STYLES: Readonly<Record<DeckSignal, SignalStyle>> = {
  event: { shape: "triangle", weight: 1.4, day: "#22271e", night: "#f3f3f0" },
  control: { shape: "dot", weight: 1, day: "#2f9e4a", night: "#46d36a" },
  audio: { shape: "square", weight: 1, day: "#0089a8", night: "#19b8d8" },
  picture: { shape: "square", weight: 1, day: "#3a63c9", night: "#6f9bff" },
  address: { shape: "diamond", weight: 1, day: "#8a4fd0", night: "#b46bff" },
  vote: { shape: "dot", weight: 1, day: "#b07800", night: "#ffb020" },
  scene: { shape: "square", weight: 1, day: "#c8385a", night: "#ff6b8a" },
  far: { shape: "ring", weight: 1, day: "#6b7461", night: "#a7af9b" },
};

/** The theme token of a signal's colour (`app.css`) */
export const signalToken = (signal: DeckSignal): string => `--signal-${signal}`;

/**
 * OSCT's signal of a port or a wire → the deck's (DESIGN.md "Logic canvas: signals"):
 * - an **event**, and a rule's verb inlet (`kind: "action"` – the exec pin of Unreal) → `event`;
 * - a **value**: a number or a whole number → `control`, true / false → `vote` (a yes or no, the deck's answer), a text,
 *   a choice or a colour → `control`;
 * - **node** (a verb acting on a node: show, hide, play …) and **address** → `address` (a place a message is sent to);
 * - **scene** (goto) → `scene`;
 * - **media**: a sound file → `audio`, a picture, a video, a texture or a model → `picture`;
 * - **far** (from outside the level) → `far`. The deck's `clock` is not taken – OSCT has no clock (Tilman, 2026-11-21).
 */
export const deckSignal = (signal: Signal, type?: PortType, kind?: PortKind): DeckSignal => {
  if (kind === "action") return "event";
  switch (signal) {
    case "event":
      return "event";
    case "value":
      return type?.kind === "boolean" ? "vote" : "control";
    case "node":
    case "address":
      return "address";
    case "scene":
      return "scene";
    case "media":
      return type?.kind === "file" && /audio|sound/.test(type.accepts) ? "audio" : "picture";
    case "far":
      return "far";
  }
};

/** The deck's grid and sizes, in px (1rem = 16px): a card's width, its head, a pin's row, the pin's radius */
export const GRID = 16;
export const CARD_W = 14 * GRID;
export const CARD_HEAD = 2 * GRID;
export const PIN_ROW = 1.5 * GRID;
/** Below this zoom a pin's label is hidden (the deck's `labels = z >= 0.55`) */
export const LABEL_ZOOM = 0.55;

/** Snap a length or a place to the grid */
export const snap = (value: number): number => Math.round(value / GRID) * GRID || 0;
/** A size grown to the next grid line */
export const snapUp = (value: number): number => Math.ceil(value / GRID) * GRID;

/** The deck's group colours, in order: a new group takes the first one no group has (deck-show `canvas.rs`) */
export const GROUP_COLOURS = ["#5fb3ff", "#7fd68a", "#f2b84b", "#e07ad6", "#ff7a6b", "#9a8cff"] as const;

/**
 * A wire hanging from `a` to `b` as the deck draws it (deck-kit `Wire::hang`): a cubic that sags under its ends, at its middle
 * 0.35rem plus a tenth of their distance below the lower end. Returns the SVG path and its lowest point (where the label sits).
 */
export const hangPath = (a: { x: number; y: number }, b: { x: number; y: number }, unit = GRID): { d: string; x: number; y: number } => {
  const depth = 0.35 * unit + 0.1 * Math.hypot(b.x - a.x, b.y - a.y);
  // A cubic's middle sits at ¾ of its inner points' offset
  const low = Math.max(a.y, b.y) + depth / 0.75;
  const c1 = { x: a.x + (b.x - a.x) / 3, y: low };
  const c2 = { x: b.x + (a.x - b.x) / 3, y: low };
  const mid = { x: (a.x + 3 * c1.x + 3 * c2.x + b.x) / 8, y: (a.y + 3 * c1.y + 3 * c2.y + b.y) / 8 };
  return { d: `M${a.x},${a.y} C${c1.x},${c1.y} ${c2.x},${c2.y} ${b.x},${b.y}`, x: mid.x, y: mid.y };
};
