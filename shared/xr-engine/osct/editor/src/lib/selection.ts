import type { EntityData } from "@engine/types";
import { nodeAt } from "./values";
import { nodeLabel } from "./tree";

/**
 * The editor's one selection (#162), shared by the tree, the stage and the Inspector: a **target** (its printed
 * image – the Inspector shows its target fields, the stage outlines the print), its **entity** (the root node) or
 * a **node** below it (the Inspector at that node, the gizmo on it). A target without AR content can only be
 * selected as the target. The tree's top rows select too (#227): the **scene** (the Inspector shows it, no target
 * or node on the stage) and an **entry** (the Inspector shows the entry; with a target, `target` is it, so the stage
 * outlines its print as for a selected target). The top bar's ⚙ after the experience's crumb selects the **experience** (#237: the Inspector shows
 * its texts and settings, nothing on the stage). Nothing selected is the scene (#229, `settledSelection`). The pure
 * rules – the level, one level up, the breadcrumb – live here, the tree's rows in `tree-rows.ts`; the shared state is
 * `selection.svelte.ts`.
 */

export type SelectionLevel = "none" | "target" | "entity" | "node";

export interface SelectionState {
  target: string | null;
  /** The node's path in the target's entity (`[]` = the root); not used while the target itself is selected */
  node: number[];
  /** The target itself is selected, not a node of its entity */
  onTarget: boolean;
  /** The scene itself is selected (the tree's top row, #227; nothing else selected, #229): no target, no node */
  scene?: boolean;
  /** An entry is selected (its id, #227); `target` is its target or null for an entry without one */
  entry?: string | null;
  /** The experience is selected (the top bar's ⚙, #237): no scene, target, node or entry */
  experience?: boolean;
  /** A group is selected (its id – a band of the Scenes panel, #307): the Inspector shows the group, nothing on the stage */
  group?: string | null;
  /**
   * A surface of a scene is selected (#314, the Surface mode, the Tree's Surfaces level): the Inspector shows it, the stage
   * highlights it – no target, node or entry
   */
  surface?: SurfaceRef | null;
}

/** A surface of a scene by ids (#314) */
export interface SurfaceRef {
  scene: string;
  id: string;
}

/**
 * A selection as the workspace keeps it (#229): the scene when nothing else is selected – no target, no entry – so the
 * tree highlights its row and the Inspector shows it; the scene selected names no target, node or entry
 */
export const settledSelection = (selection: SelectionState): SelectionState => {
  if (selection.experience) return { target: null, node: [], onTarget: true, scene: false, entry: null, experience: true };
  if (selection.group) return { target: null, node: [], onTarget: true, scene: false, entry: null, group: selection.group };
  if (selection.surface) return { target: null, node: [], onTarget: true, scene: false, entry: null, surface: { scene: selection.surface.scene, id: selection.surface.id } };
  if (selection.scene || (!selection.target && !selection.entry)) return { target: null, node: [], onTarget: true, scene: true, entry: null };
  return { ...selection, scene: false, entry: selection.entry ?? null };
};

/** The level of a selection: a target without a tree is always selected as the target */
export const selectionLevel = (selection: SelectionState, hasTree: boolean): SelectionLevel => {
  if (!selection.target) return "none";
  if (selection.onTarget || !hasTree) return "target";
  return selection.node.length ? "node" : "entity";
};

/** One level up: a node → its parent, the entity → its target, a surface → the scene; the target (an entry, the scene) stays */
export const parentSelection = (selection: SelectionState): SelectionState => {
  if (selection.surface) return { target: null, node: [], onTarget: true, scene: true };
  if (selection.experience || selection.group || selection.scene || selection.entry || !selection.target || selection.onTarget) return { ...selection, node: [] };
  if (!selection.node.length) return { target: selection.target, node: [], onTarget: true };
  return { target: selection.target, node: selection.node.slice(0, -1), onTarget: false };
};

export interface Crumb {
  label: string;
  level: Exclude<SelectionLevel, "none">;
  /** What a click on the crumb selects */
  selection: SelectionState;
  /** The selected level (the last crumb) */
  current: boolean;
}

/** The breadcrumb of a selection: target › entity › group › node, each a level to step up to */
export const breadcrumb = (selection: SelectionState, tree: EntityData | undefined): Crumb[] => {
  const target = selection.target;
  if (!target) return [];
  const level = selectionLevel(selection, !!tree);
  const crumbs: Crumb[] = [{ label: target, level: "target", selection: { target, node: [], onTarget: true }, current: level === "target" }];
  if (level === "target" || !tree) return crumbs;
  const path = nodeAt(tree, selection.node) ? selection.node : [];
  for (let depth = 0; depth <= path.length; depth++) {
    const at = path.slice(0, depth);
    const node = nodeAt(tree, at)!;
    crumbs.push({ label: nodeLabel(node), level: depth ? "node" : "entity", selection: { target, node: at, onTarget: false }, current: depth === path.length });
  }
  return crumbs;
};
