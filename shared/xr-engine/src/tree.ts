import { ADD_PLACES, addNode, defaultAddPlace, duplicateNode, groupNode, indentNode, outdentNode, shiftNode } from '../osct/editor/src/lib/tree';
import { filterTreeAdd, treeAddItems } from '../osct/editor/src/lib/tree-add';
import { addSurface } from '../osct/editor/src/lib/surface-edit';
import { newNode, nodeAt, removeNode } from '../osct/editor/src/lib/values';
import { nodeAssetTypes, nodeTypes, pluginNodeTypes } from '../osct/shared/types/node-types';
import type { SurfaceData } from '../osct/shared/types/game-config';
import type { EntityData } from '../osct/engine/src/types';
import type * as api from './tree-api';

// OSCT's tree edits as its editor's outliner and "+ node" use them (editor/src/lib/tree.ts, values.ts), unchanged.
type Tree = EntityData;
const tree = (node: api.XrNode): Tree => node as unknown as Tree;
const node = (data: Tree): api.XrNode => data as unknown as api.XrNode;
const edit = (e: { tree: Tree; path: number[] } | null): api.XrTreeEdit | null => (e ? { tree: node(e.tree), path: e.path } : null);

export const XR_ADD_PLACES: typeof api.XR_ADD_PLACES = ADD_PLACES;
export const xrNodeTypes: typeof api.xrNodeTypes = () => nodeTypes();
export const xrNodeFileTypes: typeof api.xrNodeFileTypes = (type) => {
	const kinds = nodeAssetTypes(type);
	return kinds?.length ? kinds : undefined;
};
export const xrNewNode: typeof api.xrNewNode = (type, file) => node(newNode(type as Tree['type'], file));
export const xrDefaultAddPlace: typeof api.xrDefaultAddPlace = (selected) => defaultAddPlace(selected ? tree(selected) : undefined);
export const xrAddNode: typeof api.xrAddNode = (root, selected, added, place) => edit(addNode(root ? tree(root) : root, selected, tree(added), place))!;
export const xrRemoveNode: typeof api.xrRemoveNode = (root, path) => node(removeNode(tree(root), path));
export const xrShiftNode: typeof api.xrShiftNode = (root, path, direction) => edit(shiftNode(tree(root), path, direction));
export const xrIndentNode: typeof api.xrIndentNode = (root, path) => edit(indentNode(tree(root), path));
export const xrOutdentNode: typeof api.xrOutdentNode = (root, path) => edit(outdentNode(tree(root), path));
export const xrNodeAt: typeof api.xrNodeAt = (root, path) => {
	const at = nodeAt(root ? tree(root) : undefined, path);
	return at ? node(at) : undefined;
};
export const xrDuplicateNode: typeof api.xrDuplicateNode = (root, path) => edit(duplicateNode(tree(root), path));
export const xrGroupNode: typeof api.xrGroupNode = (root, path) => edit(groupNode(tree(root), path));
export const xrTreeAddItems: typeof api.xrTreeAddItems = (context) => treeAddItems(context) as api.XrTreeAddItem[];
export const xrFilterTreeAdd: typeof api.xrFilterTreeAdd = (items, query) => filterTreeAdd(items as Parameters<typeof filterTreeAdd>[0], query) as api.XrTreeAddItem[];
export const xrPluginNodeTypes: typeof api.xrPluginNodeTypes = () => pluginNodeTypes();
export const xrAddSurface: typeof api.xrAddSurface = (preset, surfaces) => addSurface(preset, surfaces as SurfaceData[]);
