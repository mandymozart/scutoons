/**
 * The stage's pick filter (three-free), the select tool's filters (#308): what a click on the stage picks – and what the
 * pointer hovers – from what lies under the pointer, nearest first. The filters, in their order, are **entities** (an
 * entity: its root, from any of its nodes), **targets** (a printed image), **models** (the model node a hit belongs to –
 * itself or the nearest model above it) and **nodes** (the node under the pointer, as deep as the tree goes). Each is a
 * toggle saying what a click may pick; **with none on it picks anything** – the nearest hit at its most specific (the
 * node, or the print). With several on, the nearest hit any of them takes is picked, at the most specific level that is
 * on (nodes before models before entities). Nothing a filter takes under the pointer picks nothing (the workspace then
 * clears the selection). The right click's menu is the one lenient reader: the filters' pick, else the printed image
 * there, so the menu always has what lies under the pointer.
 */

export type PickKind = "entity" | "target" | "model" | "node";
/** The filters in the toolbar's order */
export const PICK_KINDS: readonly PickKind[] = ["entity", "target", "model", "node"];
/** The filters on (none: anything) */
export type PickFilter = readonly PickKind[];

/**
 * One thing under the pointer: a node of a target's entity (`path` in its tree, `[]` = the root; `model` the path of the
 * model node it belongs to, itself or the nearest above it, null when none) or a printed image
 */
export type StageHit = { kind: "node"; targetId: string; path: number[]; model?: number[] | null } | { kind: "print"; targetId: string };

/** What a click on the stage picked: a target (its printed image), or its entity's node at `path` */
export interface StagePick {
  targetId: string;
  path: number[];
  /** The printed image was picked, not an entity */
  printed?: boolean;
}

/** The filter switched: on when it was off, off when it was on – in the toolbar's order */
export const toggleKind = (filter: PickFilter, kind: PickKind): PickKind[] =>
  PICK_KINDS.filter(k => (k === kind ? !filter.includes(k) : filter.includes(k)));

/** The pick of a filter from the hits under the pointer (nearest first), null when nothing it takes lies there */
export const pickByFilter = (filter: PickFilter, hits: readonly StageHit[]): StagePick | null => {
  const any = filter.length === 0;
  for (const hit of hits) {
    if (hit.kind === "print") {
      if (any || filter.includes("target")) return { targetId: hit.targetId, path: [], printed: true };
      continue;
    }
    if (any || filter.includes("node")) return { targetId: hit.targetId, path: [...hit.path] };
    if (filter.includes("model") && hit.model) return { targetId: hit.targetId, path: [...hit.model] };
    if (filter.includes("entity")) return { targetId: hit.targetId, path: [] };
  }
  return null;
};

/** The right click's pick: the filters', else the printed image under the pointer (the menu acts on what lies there) */
export const pickForMenu = (filter: PickFilter, hits: readonly StageHit[]): StagePick | null =>
  pickByFilter(filter, hits) ?? pickByFilter(["target"], hits);

/** The path of the model node a node belongs to – itself or the nearest model above it – null when none */
export const modelOf = (path: readonly number[], isModel: (path: number[]) => boolean): number[] | null => {
  for (let n = path.length; n >= 0; n--) {
    const at = path.slice(0, n);
    if (isModel(at)) return at;
  }
  return null;
};

/** Two picks select the same thing */
export const samePick = (a: StagePick | null, b: StagePick | null): boolean =>
  a === b || (!!a && !!b && a.targetId === b.targetId && !!a.printed === !!b.printed && a.path.length === b.path.length && a.path.every((v, i) => v === b.path[i]));

/**
 * How near the pointer a particle or a line must be drawn to count (CSS px). A ray hits points and lines within a
 * threshold of a whole world unit (a target width and more in a node's space), which would let a particle cloud or a
 * line catch every click around its entity – so they count only where they are drawn.
 */
export const PICK_PX = 8;

/** A point drawn within `px` of the pointer (both in CSS px) */
export const nearPointer = (pointer: { x: number; y: number }, drawn: { x: number; y: number }, px = PICK_PX): boolean =>
  Math.hypot(pointer.x - drawn.x, pointer.y - drawn.y) <= px;

/** The part of a three.js object the pick reads: its name, visibility and parent */
export interface PickObject {
  name: string;
  visible: boolean;
  parent: PickObject | null;
}

/**
 * Whether a hit object counts: shown (itself and every parent – a target switched off hides its anchor, not the
 * meshes) and not one of the editor's own lines (outlines, a light's rays); a light's bulb is the light's to pick
 */
export const pickable = (object: PickObject): boolean => {
  if (object.name.startsWith("editor-") && object.name !== "editor-light-bulb") return false;
  for (let at: PickObject | null = object; at; at = at.parent) if (!at.visible) return false;
  return true;
};

/** The nearest node a hit object belongs to: the first of its parents (itself included) that is a node's object */
export const nodeOf = <T>(object: PickObject, nodes: ReadonlyMap<PickObject, T>): T | null => {
  for (let at: PickObject | null = object; at; at = at.parent) {
    const node = nodes.get(at);
    if (node) return node;
  }
  return null;
};
