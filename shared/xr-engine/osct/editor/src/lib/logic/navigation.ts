import type { SelectionState } from "#lib/selection.js";
import { project, type LogicContent } from "./project";
import type { Context, GraphEdge, GraphNode, LogicPick, Stub, View } from "./types";

/**
 * Moving through the levels of the Logic view (#334 L-T3, research §6.2, §6.3) – pure, so the panel only holds the one
 * `Context` and calls these:
 *
 * - **The URL** (decision L8): the level as `?logic=` – `experience`, `group:<id>`, `scene:<id>`,
 *   `entity:<scene>:<target>[:<path dotted>]`; each id percent-encoded, so an id with a `:` survives.
 * - **Back up**: the level above in `View.levels` (the breadcrumb).
 * - **The stage follows** (`stageMove`): what the workspace does when the view enters a level – open a scene, select a node,
 *   a group, the experience.
 * - **The view follows** (`followSelection`): the workspace's selection moves the view to the deepest level that shows
 *   it – unless the box it selects is already drawn here (a click on a box selects it without leaving the level).
 * - **Events between scenes** (`firesAcrossScenes`): the engine builds the entities – and so the behaviour rules – of the
 *   open scene only (`engine/src/scene/ar-scene.ts`: a scene switch disposes the old scene's entities and builds the new
 *   one's; the loading window prepares only image targets). An event a rule emits in scene A reaches the bus of A's
 *   stage; scene B's rules are not built then, so a scene-to-scene event edge never fires. Plugins load for the whole
 *   experience, so an edge to or from a plugin does.
 * - **Search** over every box of the experience and of each scene.
 */

// ── The URL (`?logic=`) ─────────────────────────────────────────────────────────────────────────────────────────

const part = (id: string) => encodeURIComponent(id);

export const encodeContext = (context: Context): string => {
  switch (context.level) {
    case "experience": return "experience";
    case "group": return `group:${part(context.group)}`;
    case "scene": return `scene:${part(context.scene)}`;
    case "entity": return `entity:${part(context.scene)}:${part(context.target)}${context.path.length ? `:${context.path.join(".")}` : ""}`;
  }
};

/** The level a `?logic=` value names; null for anything else */
export const decodeContext = (value: string | null | undefined): Context | null => {
  if (!value) return null;
  const [level, ...rest] = value.split(":");
  let parts: string[];
  try {
    parts = rest.map(decodeURIComponent);
  } catch {
    return null;
  }
  if (level === "experience" && !parts.length) return { level: "experience" };
  if (level === "group" && parts.length === 1 && parts[0]) return { level: "group", group: parts[0] };
  if (level === "scene" && parts.length === 1 && parts[0]) return { level: "scene", scene: parts[0] };
  if (level === "entity" && (parts.length === 2 || parts.length === 3) && parts[0] && parts[1]) {
    const path = parts[2] ? parts[2].split(".").map(Number) : [];
    if (path.some(n => !Number.isInteger(n) || n < 0)) return null;
    return { level: "entity", scene: parts[0], target: parts[1], path };
  }
  return null;
};

export const sameContext = (a: Context, b: Context): boolean => encodeContext(a) === encodeContext(b);

/** The scene a level belongs to (none: the experience and a group) */
export const sceneOfContext = (context: Context): string | null =>
  context.level === "scene" || context.level === "entity" ? context.scene : null;

// ── Up the levels ───────────────────────────────────────────────────────────────────────────────────────────────

/** The level above `view`'s (the breadcrumb's crumb before the last); null at the top */
export const parentLevel = (view: Pick<View, "levels">): Context | null =>
  view.levels.length > 1 ? view.levels[view.levels.length - 2] : null;

// ── The stage follows (§6.3) ────────────────────────────────────────────────────────────────────────────────────

/** What the workspace does when the view goes to a level */
export type StageMove =
  | { move: "experience" }
  | { move: "group"; group: string }
  /** Open the scene (the route) – with the scene itself selected */
  | { move: "scene"; scene: string }
  /** Select a node of a target's entity – on its scene, opened first when it is another */
  | { move: "node"; scene: string; target: string; path: number[] };

export const stageMove = (context: Context): StageMove => {
  switch (context.level) {
    case "experience": return { move: "experience" };
    case "group": return { move: "group", group: context.group };
    case "scene": return { move: "scene", scene: context.scene };
    case "entity": return { move: "node", scene: context.scene, target: context.target, path: context.path };
  }
};

// ── The view follows the selection (§6.3, the reverse) ──────────────────────────────────────────────────────────

/** Whether a box's pick stands for what is selected on scene `sceneId` (a view of another scene shows none of it) */
export const pickSelected = (pick: LogicPick | null, selection: SelectionState, sceneId: string, view: Pick<View, "context">): boolean => {
  if (!pick) return false;
  switch (pick.kind) {
    case "node": {
      const here = sceneOfContext(view.context);
      return here === sceneId && selection.target === pick.target && !selection.onTarget && selection.node.join(".") === pick.path.join(".");
    }
    case "scene": return pick.scene === sceneId && !!selection.scene;
    case "group": return !!selection.group && selection.group === pick.group;
    case "experience": return !!selection.experience;
    case "plugin": return false;
  }
};

/** The deepest level that shows the selection: a group, the experience, a target's entity, else the open scene */
export const levelOfSelection = (selection: SelectionState, sceneId: string): Context => {
  if (selection.experience) return { level: "experience" };
  if (selection.group) return { level: "group", group: selection.group };
  if (selection.target && !selection.entry) return { level: "entity", scene: sceneId, target: selection.target, path: [] };
  return { level: "scene", scene: sceneId };
};

/**
 * The level the view moves to when the selection changes: it stays while one of its boxes stands for the selection (or,
 * on a target's entity level, while a node of that subtree is selected), else it goes to `levelOfSelection`.
 */
export const followSelection = (current: View, selection: SelectionState, sceneId: string): Context => {
  if (current.nodes.some(n => pickSelected(n.pick, selection, sceneId, current))) return current.context;
  const c = current.context;
  if (c.level === "entity" && c.scene === sceneId && selection.target === c.target && !selection.entry
    && (selection.onTarget || c.path.every((p, i) => selection.node[i] === p))) return c;
  return levelOfSelection(selection, sceneId);
};

// ── Events between scenes ───────────────────────────────────────────────────────────────────────────────────────

const sceneBox = (id: string) => id.startsWith("scene:");

/** An event edge between two scene boxes (the group and experience levels): it never fires – see the module's comment */
export const neverFires = (edge: Pick<GraphEdge, "signal" | "from" | "to">): boolean =>
  edge.signal === "event" && sceneBox(edge.from[0]) && sceneBox(edge.to[0]) && edge.from[0] !== edge.to[0];

/** A stub carrying an event to or from another scene: it never fires either */
export const stubNeverFires = (stub: Stub, view: Pick<View, "context">): boolean =>
  stub.edge.signal === "event" && stub.far.level === "scene" && stub.far.scene !== sceneOfContext(view.context);

// ── Search ──────────────────────────────────────────────────────────────────────────────────────────────────────

export interface SearchHit {
  /** The box's id in the view of `context` */
  node: string;
  kind: string;
  label: string;
  detail: string;
  /** The level that draws the box */
  context: Context;
  /** Where that is, in words: `Experience`, the scene's label */
  where: string;
}

/** Every box of the experience level and of each scene level, once per level and box */
export const searchIndex = (content: LogicContent): SearchHit[] => {
  const hits: SearchHit[] = [];
  const add = (view: View, where: string) => {
    for (const n of view.nodes) hits.push({ node: n.id, kind: n.kind, label: n.label, detail: n.detail, context: view.context, where });
  };
  add(project(content, { level: "experience" }), "Experience");
  for (const scene of content.scenes) add(project(content, { level: "scene", scene: scene.id }), scene.label);
  return hits;
};

/** The hits whose label (or detail) holds every word of `query`, labels first; at most `limit` */
export const searchLogic = (index: readonly SearchHit[], query: string, limit = 12): SearchHit[] => {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const inLabel: SearchHit[] = [];
  const inDetail: SearchHit[] = [];
  for (const hit of index) {
    const label = hit.label.toLowerCase();
    const all = `${label} ${hit.detail.toLowerCase()} ${hit.where.toLowerCase()}`;
    if (words.every(w => label.includes(w))) inLabel.push(hit);
    else if (words.every(w => all.includes(w))) inDetail.push(hit);
  }
  return [...inLabel, ...inDetail].slice(0, limit);
};

// ── The breadcrumb's words ──────────────────────────────────────────────────────────────────────────────────────

/** A level's name in the breadcrumb: `Experience`, the group's and the scene's label, the target's id, a node's path */
export const levelName = (content: LogicContent, context: Context): string => {
  switch (context.level) {
    case "experience": return "Experience";
    case "group": return content.groups?.find(g => g.id === context.group)?.label ?? context.group;
    case "scene": return content.scenes.find(s => s.id === context.scene)?.label ?? context.scene;
    case "entity": return context.path.length ? `${context.target} › ${context.path.join(".")}` : context.target;
  }
};

/** A box that can be entered (double-click or its enter button) */
export const canEnter = (node: Pick<GraphNode, "enter">): node is GraphNode & { enter: Context } => !!node.enter;
