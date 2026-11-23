/**
 * A scene's and the experience's logic (content schema 5.0, #334, agents/research/2026-11-20-logic-nodes-shared-concept.md
 * §5.4): rules beside the nodes' own behaviour, in the same grammar (`./behaviour.ts`):
 *
 *   # scenes/<id>/scene.yaml – live while the scene is open
 *   logic:
 *     rules:
 *       - on: doorOpened
 *         do: { show: "target:door/hint" }
 *
 *   # experience.yaml – live for the whole session, across scene switches
 *   logic:
 *     rules:
 *       - on: targetUnlocked
 *         when: hasKey
 *         do: [{ set: { chapterDone: true }, scope: experience }, { goto: "scene:epilogue" }]
 *
 * `rules` listen to the bus's events (scene events, core events – not a node's own found / lost / tap / unlocked) and name
 * nodes by address (`target:<targetId>/<node name>`, `./address.ts`). `canvas` is authoring data for the editor's Logic
 * view (`LogicCanvas`): stored with the experience (the database, the editor, the export) and never in a reader's game
 * configuration – the content build and a publish strip it (`withoutCanvas`), and the engine never reads it.
 */

import { isObject } from "../guards/primitives";
import { type BehaviourRule, readRules, type RuleHome, rulesData, rulesProblems } from "./behaviour";

/**
 * The Logic view's authoring data beside a list of rules (the editor builds its UI later, #334 L-T7) – kept small:
 * - `groups`: a frame around boxes of the view (`members`: the view's node ids), with a label;
 * - `notes`: a text on the canvas (`LogicCanvasNote`) – free at a point of the view, or beside a box (schema 5.3: `box`,
 *   `author`, `view`);
 * - `folds`: the view's node ids that are folded to a pill;
 * - `exposed` (schema 5.1): the ports an author exposed on a box beyond its wired ones – box id → port ids. A box shows its
 *   wired ports and these; the rest of the ports its spec gives stay hidden behind its **+** (the Blender way, #334).
 * - `positions` (schema 5.2, L-T7): where an author pinned a box over the automatic layout – box id → `{ x, y }` in the
 *   view's px (on the 1rem = 16px grid). A box without one sits where the layout puts it; the editor's *Tidy* drops them.
 */
export interface LogicCanvas {
  groups?: LogicCanvasGroup[];
  notes?: LogicCanvasNote[];
  folds?: string[];
  exposed?: Record<string, string[]>;
  positions?: Record<string, LogicCanvasPoint>;
}

/** A pinned box's place on the canvas (schema 5.2) */
export interface LogicCanvasPoint {
  x: number;
  y: number;
}

/**
 * A frame around boxes (as the SCUTOONS deck's groups): `colour` (schema 5.2) the `#rrggbb` it is drawn in, `folded`
 * (schema 5.2) the group drawn as one pill instead of its boxes
 */
export interface LogicCanvasGroup {
  id: string;
  label: string;
  members: string[];
  colour?: string;
  folded?: boolean;
}

/**
 * A note on the canvas – an annotation, after the SCUTOONS deck's notes (#334): never logic, the engine and the reader
 * ignore it. `x`, `y`: a free note's place in the view's px; a note with `box` (schema 5.3) stands beside that box and
 * moves with it – `x`, `y` are then its offset from the box's top-left corner. `author` (schema 5.3): who wrote it, as the
 * editor names them. `view` (schema 5.3): the target's view it is drawn in when its canvas is a scene's – `<target>` or
 * `<target>/<node path>` (`door/0.1`); without one it is drawn at the scene's (or the experience's) own level.
 */
export interface LogicCanvasNote {
  id: string;
  text: string;
  x: number;
  y: number;
  author?: string;
  box?: string;
  view?: string;
}

/** A scene's or the experience's logic in the content */
export interface LogicData {
  /** Rules in the content's form (`./behaviour.ts`): `{ on, when?, do }` – read them with `logicRules` */
  rules: Record<string, unknown>[];
  /** Authoring data of the editor's Logic view – never in a reader's configuration */
  canvas?: LogicCanvas;
}

/** The homes a `logic` field has */
export type LogicHome = Exclude<RuleHome, "entity">;

const isText = (value: unknown): value is string => typeof value === "string";
const isNumber = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);

/** The fields a canvas has (`exposed` since schema 5.1, `positions` since 5.2) */
const CANVAS_KEYS = ["groups", "notes", "folds", "exposed", "positions"];
/** The fields a note has (`author`, `box` and `view` since schema 5.3) */
const NOTE_KEYS = ["id", "text", "x", "y", "author", "box", "view"];
/** A group's colour: `#rrggbb` */
const COLOUR = /^#[0-9a-fA-F]{6}$/;

/** Problems of a canvas (`at` = its path) */
export const canvasProblems = (canvas: unknown, at: string): string[] => {
  if (!isObject(canvas)) return [`${at}: expected { groups?, notes?, folds?, exposed?, positions? }`];
  const problems: string[] = [];
  Object.keys(canvas).filter(key => !CANVAS_KEYS.includes(key)).forEach(key => problems.push(`${at}.${key}: unknown field of a canvas (${CANVAS_KEYS.join(", ")})`));
  const list = (key: string, check: (item: unknown, path: string) => string[]) => {
    const value = canvas[key];
    if (value === undefined) return;
    if (!Array.isArray(value)) return problems.push(`${at}.${key}: expected a list`);
    value.forEach((item, i) => problems.push(...check(item, `${at}.${key}[${i}]`)));
  };
  list("groups", (group, path) => {
    if (!isObject(group) || !isText(group.id) || !group.id || !isText(group.label) || !Array.isArray(group.members) || !group.members.every(isText)) {
      return [`${path}: expected { id, label, members: [node ids] }`];
    }
    const wrong: string[] = [];
    if (group.colour !== undefined && !(isText(group.colour) && COLOUR.test(group.colour))) wrong.push(`${path}.colour: expected a colour #rrggbb`);
    if (group.folded !== undefined && typeof group.folded !== "boolean") wrong.push(`${path}.folded: expected true or false`);
    return wrong;
  });
  list("notes", (note, path) => {
    if (!isObject(note) || !isText(note.id) || !note.id || !isText(note.text) || !isNumber(note.x) || !isNumber(note.y)) return [`${path}: expected { id, text, x, y }`];
    const wrong: string[] = [];
    Object.keys(note).filter(key => !NOTE_KEYS.includes(key)).forEach(key => wrong.push(`${path}.${key}: unknown field of a note (${NOTE_KEYS.join(", ")})`));
    for (const key of ["author", "box", "view"]) {
      if (note[key] !== undefined && !(isText(note[key]) && note[key])) wrong.push(`${path}.${key}: expected a text`);
    }
    return wrong;
  });
  list("folds", (fold, path) => (isText(fold) && fold ? [] : [`${path}: expected a node id`]));
  if (canvas.exposed !== undefined) {
    if (!isObject(canvas.exposed)) problems.push(`${at}.exposed: expected { <box id>: [port ids] }`);
    else {
      for (const [box, ports] of Object.entries(canvas.exposed)) {
        if (!box || !Array.isArray(ports) || !ports.every(port => isText(port) && port)) problems.push(`${at}.exposed.${box}: expected a list of port ids`);
      }
    }
  }
  if (canvas.positions !== undefined) {
    if (!isObject(canvas.positions)) problems.push(`${at}.positions: expected { <box id>: { x, y } }`);
    else {
      for (const [box, point] of Object.entries(canvas.positions)) {
        if (!box || !isObject(point) || !isNumber(point.x) || !isNumber(point.y)) problems.push(`${at}.positions.${box}: expected { x, y }`);
      }
    }
  }
  return problems;
};

/** Problems of a scene's or the experience's `logic` (`at` = its path) – the content build, the guard */
export const logicProblems = (logic: unknown, at: string, home: LogicHome): string[] => {
  if (!isObject(logic)) return [`${at}: expected { rules, canvas? }`];
  const problems: string[] = [];
  Object.keys(logic).filter(key => !["rules", "canvas"].includes(key)).forEach(key => problems.push(`${at}.${key}: unknown field of logic (rules, canvas)`));
  if (logic.rules === undefined) problems.push(`${at}.rules: expected a list of rules { on, when?, do }`);
  else problems.push(...rulesProblems(logic.rules, `${at}.rules`, home));
  if (logic.canvas !== undefined) problems.push(...canvasProblems(logic.canvas, `${at}.canvas`));
  return problems;
};

/** The rules of a `logic` field as the code works with them ([] when there is none) */
export const logicRules = (logic: unknown): BehaviourRule[] => (isObject(logic) ? readRules(logic.rules) : []);

/** A `logic` field in the content's shortest form – what the editor saves; undefined when it holds nothing */
export const logicData = (rules: readonly BehaviourRule[], canvas?: LogicCanvas): LogicData | undefined =>
  rules.length || canvas ? { rules: rulesData(rules), ...(canvas ? { canvas } : {}) } : undefined;

/**
 * A value without its logic's canvas – a reader's configuration never carries authoring data. Returns the same object
 * when it has none (a scene, the experience)
 */
export const withoutCanvas = <T extends { logic?: LogicData }>(value: T): T => {
  if (!value.logic || value.logic.canvas === undefined) return value;
  const { canvas: _canvas, ...logic } = value.logic;
  return { ...value, logic };
};
