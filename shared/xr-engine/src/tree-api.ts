/**
 * What an app sees of OSCT's tree edits (editor/src/lib/tree.ts, values.ts): the types of `./tree`, with no import of
 * OSCT's files (as reader-api.ts for `./reader`). src/tree.ts is the code and must fit these declarations.
 *
 * A target's entity is one node tree (`type`, `children`, its files in `assets`, its placement in `params` …). Every
 * function returns a new tree and the path of the node it placed (`[]` = the root); the tree passed in stays as it is.
 */

/** a node of an entity's tree as the configuration stores it */
export interface XrNode {
	type: string;
	name?: string;
	assets?: { id: string; assetType: XrAssetType; src: string }[];
	children?: XrNode[];
	[key: string]: unknown;
}

/** the file types a node takes */
export type XrAssetType = 'glb' | 'gltf' | 'video' | 'image' | 'audio';

/** where a new node goes: next to the selected one (the same parent) or inside it (its last child) */
export type XrAddPlace = 'next' | 'inside';
export declare const XR_ADD_PLACES: readonly XrAddPlace[];

/** a changed tree and the path of the node that was placed */
export interface XrTreeEdit {
	tree: XrNode;
	path: number[];
}

/** the built-in node types, then those the experience's plugins registered (after the stage loaded them) */
export declare function xrNodeTypes(): string[];

/** the file types a node of this type takes; undefined when it takes no file */
export declare function xrNodeFileTypes(type: string): readonly XrAssetType[] | undefined;

/** a new node of the type, with its file when it takes one (a text starts with the entry's title, white) */
export declare function xrNewNode(type: string, file?: { id: string; assetType: XrAssetType; src: string }): XrNode;

/** the default place of a new node: next to a node that holds a file, else inside the selected one */
export declare function xrDefaultAddPlace(selected: XrNode | undefined): XrAddPlace;

/**
 * `node` added at the selected path: inside it (its last child) or next to it (right after it). No tree yet: the node is
 * the root. Next to the root wraps the root in a group first, so both become siblings.
 */
export declare function xrAddNode(root: XrNode | null | undefined, selected: number[], node: XrNode, place: XrAddPlace): XrTreeEdit;

/** the tree without the node at `path` (never the root) */
export declare function xrRemoveNode(root: XrNode, path: number[]): XrNode;

/** up / down among its siblings (null at the end) */
export declare function xrShiftNode(root: XrNode, path: number[], direction: -1 | 1): XrTreeEdit | null;

/** the node becomes the last child of the sibling above it (null for a first child or the root) */
export declare function xrIndentNode(root: XrNode, path: number[]): XrTreeEdit | null;

/** the node goes right after its parent (a child of the root next to the root – the root is wrapped in a group) */
export declare function xrOutdentNode(root: XrNode, path: number[]): XrTreeEdit | null;

/** the node at `path` (undefined: none there) */
export declare function xrNodeAt(root: XrNode | undefined, path: number[]): XrNode | undefined;

/** a copy of the node with its children right after it: the copy's names free ones (door → door 2), its ids gone */
export declare function xrDuplicateNode(root: XrNode, path: number[]): XrTreeEdit | null;

/** the node into a new group in its place (it stays where it is on the stage); the root grouped: the group is the root */
export declare function xrGroupNode(root: XrNode, path: number[]): XrTreeEdit | null;

/** what the Tree's + adds (OSCT's tree-add.ts): a surface of a preset, a target or an entry (their forms), a node of a type */
export type XrTreeAdd = { add: 'surface'; preset: XrSurfacePreset } | { add: 'target' } | { add: 'entry' } | { add: 'node'; type: string };
export type XrSurfacePreset = 'double' | 'single' | 'wall' | 'custom';

export interface XrTreeAddItem {
	key: string;
	label: string;
	detail: string;
	/** `scene`, `nodes` or `plugin nodes` */
	group: string;
	/** why it cannot be added now; null when it can */
	disabled: string | null;
	add: XrTreeAdd;
}

export interface XrTreeAddContext {
	/** the target a node would hang on (the selected target, or the selected node's); null for none */
	target: string | null;
	/** that target has content already: the node goes next to / inside the selected node, else it is the root */
	hasEntity: boolean;
	/** the selected node's name, when a node is selected */
	node: string | null;
	builtIn: readonly string[];
	plugins: readonly string[];
	takesFile: (type: string) => boolean;
	canAddTarget: boolean;
	canAddEntry: boolean;
}

/** every item of the + menu for the context: the scene's (surfaces, target, entry), the node types, the plugins' */
export declare function xrTreeAddItems(context: XrTreeAddContext): XrTreeAddItem[];

/** the items whose label, group or detail hold every typed word */
export declare function xrFilterTreeAdd(items: readonly XrTreeAddItem[], query: string): XrTreeAddItem[];

/** the plugins' node types (after the stage loaded the experience's plugins) */
export declare function xrPluginNodeTypes(): string[];

/** a scene's surfaces with a new one of the preset (beside the row, a wall behind it): the list and the new surface */
export declare function xrAddSurface(preset: XrSurfacePreset, surfaces: readonly unknown[]): { surfaces: unknown[]; added: { id: string } };
