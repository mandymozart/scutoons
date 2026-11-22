/**
 * The Tree's **+** (#337): one menu of everything that can be added to the open scene – a surface (a double or single
 * page, a wall, a custom one), a target and an entry (their forms), and a node of every type the editor makes (the
 * built-in ones, then the experience's plugins'). It is context-aware: a node needs a target to hang on – the selected
 * target, the target of the selected entry, or the selected node's (the new node goes where the Inspector's *+ node*
 * puts it: the target's root when it has no AR content, else next to the selected node or inside it); without one it is
 * offered greyed out with the reason. A type that takes a file (a model, a video, an image) asks for it in the Library
 * next. Pure: the Tree turns a pick into the action.
 */
import { SURFACE_PRESETS, type SurfacePreset } from "./surface-edit";

export type TreeAdd =
  | { add: "surface"; preset: SurfacePreset }
  | { add: "target" }
  | { add: "entry" }
  | { add: "node"; type: string };

export interface TreeAddItem {
  key: string;
  label: string;
  /** What it does, in a few words (the row's tooltip and grey line) */
  detail: string;
  group: string;
  /** Why it cannot be added now, null when it can */
  disabled: string | null;
  add: TreeAdd;
}

/** Where the menu opens: what is selected, what the workspace can open */
export interface TreeAddContext {
  /** The target a node would hang on (the selected target, the selected entry's, the selected node's), null for none */
  target: string | null;
  /** That target has AR content already: the node goes next to / inside the selected node, else it is the root */
  hasEntity: boolean;
  /** The selected node's name or path in words, when a node is selected */
  node: string | null;
  /** The built-in node types, then the plugins' */
  builtIn: readonly string[];
  plugins: readonly string[];
  /** The node type takes a file */
  takesFile: (type: string) => boolean;
  /** The workspace opens the new-target / new-entry form */
  canAddTarget: boolean;
  canAddEntry: boolean;
}

const SURFACE_WORDS: Record<SurfacePreset, { label: string; detail: string }> = {
  double: { label: "double page", detail: "a surface of 460 × 320 mm, flat beside the others" },
  single: { label: "single page", detail: "a surface of 230 × 320 mm, flat beside the others" },
  wall: { label: "wall", detail: "a surface of 2000 × 1500 mm, standing ahead behind the pages" },
  custom: { label: "custom surface", detail: "a surface of 300 × 300 mm – set its size in the Inspector" },
};

const NO_TARGET = "Select a target, its entry or a node first – a node hangs on a target";

/** Every item of the menu for the context, in its groups (scene, nodes, plugin nodes) */
export const treeAddItems = (ctx: TreeAddContext): TreeAddItem[] => {
  const where = !ctx.target ? "" : !ctx.hasEntity ? `the root of ${ctx.target}` : ctx.node ? `beside or in ${ctx.node}` : `in ${ctx.target}'s AR content`;
  const node = (type: string, group: string): TreeAddItem => ({
    key: `node:${type}`,
    label: type,
    detail: `${where}${ctx.takesFile(type) ? " – then its file from the Library" : ""}`,
    group,
    disabled: ctx.target ? null : NO_TARGET,
    add: { add: "node", type },
  });
  return [
    ...SURFACE_PRESETS.map((preset): TreeAddItem => ({
      key: `surface:${preset}`, label: SURFACE_WORDS[preset].label, detail: SURFACE_WORDS[preset].detail, group: "scene", disabled: null, add: { add: "surface", preset },
    })),
    {
      key: "target", label: "target…", detail: "an entry with a printed image – opens its form", group: "scene",
      disabled: ctx.canAddTarget ? null : "Not here", add: { add: "target" },
    },
    { key: "entry", label: "entry…", detail: "an entry without a printed image – opens its form", group: "scene", disabled: ctx.canAddEntry ? null : "Not here", add: { add: "entry" } },
    ...ctx.builtIn.map(type => node(type, "nodes")),
    ...ctx.plugins.filter(type => !ctx.builtIn.includes(type)).map(type => node(type, "plugin nodes")),
  ];
};

/** The items a typed filter keeps: its words in the label, the group or the detail, case ignored */
export const filterTreeAdd = (items: readonly TreeAddItem[], query: string): TreeAddItem[] => {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [...items];
  return items.filter(item => {
    const text = `${item.label} ${item.group} ${item.detail}`.toLowerCase();
    return words.every(w => text.includes(w));
  });
};
