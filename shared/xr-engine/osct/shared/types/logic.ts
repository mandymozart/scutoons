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
 * `rules` hear the bus's events (scene events, core events – not a node's own found / lost / tap / unlocked) and name
 * nodes by address (`target:<targetId>/<node name>`, `./address.ts`). `canvas` is authoring data for the editor's Logic
 * view (`LogicCanvas`): stored with the experience (the database, the editor, the export) and never in a reader's game
 * configuration – the content build and a publish strip it (`withoutCanvas`), and the engine never reads it.
 */

import { isObject } from "../guards/primitives";
import { type BehaviourRule, readRules, type RuleHome, rulesData, rulesProblems } from "./behaviour";

/**
 * The Logic view's authoring data beside a list of rules (the editor builds its UI later, #334 L-T7) – kept small:
 * - `groups`: a frame around boxes of the view (`members`: the view's node ids), with a label;
 * - `notes`: a free text on the canvas at a point of the view (x, y in the view's units);
 * - `folds`: the view's node ids that are folded to a pill;
 * - `exposed` (schema 5.1): the ports an author exposed on a box beyond its wired ones – box id → port ids. A box shows its
 *   wired ports and these; the rest of the ports its spec gives stay hidden behind its **+** (the Blender way, #334).
 */
export interface LogicCanvas {
  groups?: LogicCanvasGroup[];
  notes?: LogicCanvasNote[];
  folds?: string[];
  exposed?: Record<string, string[]>;
}

export interface LogicCanvasGroup {
  id: string;
  label: string;
  members: string[];
}

export interface LogicCanvasNote {
  id: string;
  text: string;
  x: number;
  y: number;
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

/** The fields a canvas has (`exposed` since schema 5.1) */
const CANVAS_KEYS = ["groups", "notes", "folds", "exposed"];

/** Problems of a canvas (`at` = its path) */
export const canvasProblems = (canvas: unknown, at: string): string[] => {
  if (!isObject(canvas)) return [`${at}: expected { groups?, notes?, folds?, exposed? }`];
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
    return [];
  });
  list("notes", (note, path) =>
    !isObject(note) || !isText(note.id) || !note.id || !isText(note.text) || !isNumber(note.x) || !isNumber(note.y) ? [`${path}: expected { id, text, x, y }`] : []);
  list("folds", (fold, path) => (isText(fold) && fold ? [] : [`${path}: expected a node id`]));
  if (canvas.exposed !== undefined) {
    if (!isObject(canvas.exposed)) problems.push(`${at}.exposed: expected { <box id>: [port ids] }`);
    else {
      for (const [box, ports] of Object.entries(canvas.exposed)) {
        if (!box || !Array.isArray(ports) || !ports.every(port => isText(port) && port)) problems.push(`${at}.exposed.${box}: expected a list of port ids`);
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
