// Small helpers to read/write nested config values by path (['groups', 0, 'slices', 3, 'dialog', 1, 'rect']).

export type Path = (string | number)[];

export function getAt(root: any, path: Path): any {
	let node = root;
	for (const key of path) {
		if (node === null || node === undefined) return undefined;
		node = node[key as any];
	}
	return node;
}

/** Sets a value, creating missing objects/arrays on the way. `undefined` deletes the key. */
export function setAt(root: any, path: Path, value: any): void {
	if (!path.length) throw new Error('setAt: empty path');
	let node = root;
	for (let i = 0; i < path.length - 1; i++) {
		const key = path[i];
		if (node[key as any] === undefined || node[key as any] === null || typeof node[key as any] !== 'object') {
			node[key as any] = typeof path[i + 1] === 'number' ? [] : {};
		}
		node = node[key as any];
	}
	const last = path[path.length - 1];
	if (value === undefined) {
		if (Array.isArray(node) && typeof last === 'number') node.splice(last, 1);
		else delete node[last as any];
	} else {
		node[last as any] = value;
	}
}

/** Removes `path` and then every parent object that became empty (up to `stopAt` levels from the root). */
export function deleteAt(root: any, path: Path, stopAt = 0): void {
	setAt(root, path, undefined);
	for (let depth = path.length - 1; depth > stopAt; depth--) {
		const parentPath = path.slice(0, depth);
		const parent = getAt(root, parentPath);
		if (parent && typeof parent === 'object' && !Array.isArray(parent) && Object.keys(parent).length === 0) {
			setAt(root, parentPath, undefined);
		} else {
			break;
		}
	}
}

export function clone<T>(value: T): T {
	return value === undefined ? value : JSON.parse(JSON.stringify(value));
}

export function deepEqual(a: any, b: any): boolean {
	if (a === b) return true;
	if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
	if (Array.isArray(a) !== Array.isArray(b)) return false;
	if (Array.isArray(a)) {
		if (a.length !== b.length) return false;
		for (let i = 0; i < a.length; i++) if (!deepEqual(a[i], b[i])) return false;
		return true;
	}
	const ka = Object.keys(a).filter((k) => a[k] !== undefined);
	const kb = Object.keys(b).filter((k) => b[k] !== undefined);
	if (ka.length !== kb.length) return false;
	for (const k of ka) if (!deepEqual(a[k], b[k])) return false;
	return true;
}

export const pathKey = (path: Path) => path.join('.');

/** 2 decimals, like the hand-written configs */
export const round2 = (value: number) => Math.round(value * 100) / 100;
