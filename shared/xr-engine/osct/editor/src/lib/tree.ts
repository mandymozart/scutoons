import { resolveLight, type EntityData } from "@engine/types";
import { nodeAt, removeNode, withNode } from "./values";

/**
 * The hierarchy of an entity's node tree as the editor's outliner changes it (#163): add a node next to the selected
 * one or inside it, move a node to another parent or place (drag and drop in the tree, or the keys), and the name a
 * row shows. Only parent and order – no constraints, and a moved node keeps its placement values (relative to its
 * new parent). Every function returns a new tree and the path of the node it placed; the tree passed in stays as it
 * is. The root is one node: placing a node **next to the root** wraps the root in a group first, so both become
 * siblings under the target (the group sits where the root sat – a group's placement defaults to none).
 */

/** Where "+ node" puts the new node: next to the selected one (the same parent) or inside it (its last child) */
export type AddPlace = "next" | "inside";
export const ADD_PLACES: readonly AddPlace[] = ["next", "inside"];
/** Nodes that hold a file rarely have children: a new node goes next to them by default */
export const FILE_NODE_TYPES: readonly EntityData["type"][] = ["model", "video", "image"];

/** Where a dragged row lands on another: before it, inside it (its last child) or after it */
export type DropZone = "before" | "inside" | "after";

/** A changed tree and the path of the node that was placed */
export interface TreeEdit {
  tree: EntityData;
  path: number[];
}

export const samePath = (a: readonly number[], b: readonly number[]): boolean => a.length === b.length && a.every((v, i) => v === b[i]);

/** `path` is `ancestor` or lies below it */
export const isWithin = (path: readonly number[], ancestor: readonly number[]): boolean =>
  path.length >= ancestor.length && ancestor.every((v, i) => v === path[i]);

/** The default place of a new node: next to a node that holds a file, else inside the selected node */
export const defaultAddPlace = (selected: EntityData | undefined): AddPlace =>
  selected && FILE_NODE_TYPES.includes(selected.type) ? "next" : "inside";

/** The root inside a new group, so that it can have a sibling */
export const wrapRoot = (root: EntityData): EntityData => ({ type: "group", assets: [], children: [root] });

/** A copy of the tree with `node` among the children of the node at `parent`, at `index` (clamped) */
export const insertAt = (root: EntityData, parent: number[], index: number, node: EntityData): EntityData => {
  const at = nodeAt(root, parent) ?? root;
  const children = [...(at.children ?? [])];
  children.splice(Math.max(0, Math.min(index, children.length)), 0, node);
  return withNode(root, parent, { ...at, children });
};

/**
 * Add `node` to the tree at the selected path: inside it (as its last child) or next to it (right after it, the
 * same parent). No tree yet: the node becomes the root. A selected path the tree no longer has counts as the root.
 */
export const addNode = (root: EntityData | null | undefined, selected: number[], node: EntityData, place: AddPlace): TreeEdit => {
  if (!root) return { tree: node, path: [] };
  const at = nodeAt(root, selected) ? selected : [];
  if (place === "inside") {
    const index = nodeAt(root, at)?.children?.length ?? 0;
    return { tree: insertAt(root, at, index, node), path: [...at, index] };
  }
  if (!at.length) return { tree: insertAt(wrapRoot(root), [], 1, node), path: [1] };
  const parent = at.slice(0, -1);
  const index = at[at.length - 1] + 1;
  return { tree: insertAt(root, parent, index, node), path: [...parent, index] };
};

/**
 * Move the node at `from` onto the node at `onto`: inside it (its last child), or before / after it (its sibling,
 * the order set). Null when the move is impossible (the root itself, onto the node or below it, a path the tree does
 * not have) or changes nothing. Before or after the root wraps the root in a group.
 */
export const moveNode = (root: EntityData, from: number[], onto: number[], zone: DropZone): TreeEdit | null => {
  if (!from.length || isWithin(onto, from)) return null;
  const node = nodeAt(root, from);
  if (!node || !nodeAt(root, onto)) return null;
  if (!onto.length && zone !== "inside") {
    return moveNode(wrapRoot(root), [0, ...from], [0], zone);
  }
  // The place in the tree as it is: the parent and the index among its children
  let parent = zone === "inside" ? [...onto] : onto.slice(0, -1);
  let index = zone === "inside" ? (nodeAt(root, onto)?.children?.length ?? 0) : onto[onto.length - 1] + (zone === "after" ? 1 : 0);
  // The same place in the tree without the node: its later siblings (and what lies below them) move up by one
  const depth = from.length - 1;
  const fromParent = from.slice(0, depth);
  const fromIndex = from[depth];
  if (samePath(parent, fromParent)) {
    if (index > fromIndex) index -= 1;
    if (index === fromIndex) return null;
  } else if (parent.length > depth && isWithin(parent, fromParent) && parent[depth] > fromIndex) {
    parent = [...parent];
    parent[depth] -= 1;
  }
  return { tree: insertAt(removeNode(root, from), parent, index, node), path: [...parent, index] };
};

/** Indent: the node becomes the last child of the sibling before it (null for a first child or the root) */
export const indentNode = (root: EntityData, path: number[]): TreeEdit | null => {
  const last = path[path.length - 1];
  return path.length && last > 0 ? moveNode(root, path, [...path.slice(0, -1), last - 1], "inside") : null;
};

/** Outdent: the node goes right after its parent (a child of the root next to the root – the root is wrapped) */
export const outdentNode = (root: EntityData, path: number[]): TreeEdit | null =>
  path.length ? moveNode(root, path, path.slice(0, -1), "after") : null;

/** Up / down among its siblings (null at the end) */
export const shiftNode = (root: EntityData, path: number[], direction: -1 | 1): TreeEdit | null => {
  if (!path.length) return null;
  const last = path[path.length - 1];
  const siblings = nodeAt(root, path.slice(0, -1))?.children?.length ?? 0;
  const next = last + direction;
  if (next < 0 || next >= siblings) return null;
  return moveNode(root, path, [...path.slice(0, -1), next], direction < 0 ? "before" : "after");
};

/** The paths of the rows a tree shows, depth first: the children of a folded node are not shown */
export const visiblePaths = (root: EntityData, isOpen: (path: number[]) => boolean, path: number[] = []): number[][] => [
  path,
  ...(root.children?.length && isOpen(path) ? root.children.flatMap((child, i) => visiblePaths(child, isOpen, [...path, i])) : []),
];

/** What a node is when it has no name: its type (a light's kind, a text's line), and its id when it has one */
export const typeLabel = (node: EntityData): string => {
  if (node.type === "text") return `“${(node.text ?? "").slice(0, 40)}”`;
  const type = node.type === "light" ? `${resolveLight(node.light).kind} light` : node.type;
  return `${type}${node.id ? ` ${node.id}` : ""}`;
};

/** The name a row shows: the node's own name, else what it is */
export const nodeLabel = (node: EntityData): string => node.name?.trim() || typeLabel(node);

/** A name as the content stores it: trimmed, none when empty */
export const withName = (node: EntityData, name: string): EntityData => {
  const next: EntityData = { ...node };
  const trimmed = name.trim();
  if (trimmed) next.name = name;
  else delete next.name;
  return next;
};
