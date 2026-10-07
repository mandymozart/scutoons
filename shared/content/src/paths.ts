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

/**
 * A toon document's media path → its path from the CDN root (agents/ASSETS.md "Paths in content", BACKLOG 10.39) – THE
 * one place both apps (reader + creator) resolve stored media paths: a plain path is relative to the toon's folder
 * (`s1/c1/x.webp` in catpope → `catpope/s1/c1/x.webp`); `../` steps out of it (`../catpope/s1/x.webp` in toptoy →
 * `catpope/s1/x.webp` – what the asset picker writes for another toon's file and what a paste into another toon writes);
 * a leading `/` starts at the CDN root (`/characters/x.webp`, the VN character views). `.` / `..` segments are resolved
 * here, so no URL depends on the browser normalising them. Absolute URLs (`https:`, `//`, `blob:`) stay as they are.
 */
export function toonAsset(toon: string | null | undefined, path: string | null | undefined): string {
	if (!path || typeof path !== 'string') return '';
	if (/^([a-z][a-z0-9+.-]*:|\/\/)/i.test(path)) return path;
	const fromRoot = path.startsWith('/');
	const parts: string[] = [];
	for (const segment of `${fromRoot || !toon ? '' : `${toon}/`}${path.replace(/^\/+/, '')}`.split('/')) {
		if (!segment || segment === '.') continue;
		if (segment === '..') parts.pop();
		else parts.push(segment);
	}
	return parts.join('/');
}

/**
 * The reverse of `toonAsset`: a path from the CDN root as `toon`'s documents store it – inside the toon's folder
 * relative to it, else with `../` (`catpope/s1/x.webp` in toptoy → `../catpope/s1/x.webp`).
 */
export function toonAssetRef(toon: string, rootPath: string): string {
	const clean = rootPath.replace(/^\/+/, '');
	if (!toon) return clean;
	return clean.startsWith(`${toon}/`) ? clean.slice(toon.length + 1) : `../${clean}`;
}

/** a media path of toon `from`'s documents as toon `to`'s documents store it (a paste across toons, BACKLOG 10.39) */
export function rebaseToonAsset(path: string, from: string, to: string): string {
	if (!path || from === to || /^([a-z][a-z0-9+.-]*:|\/\/)/i.test(path)) return path;
	return toonAssetRef(to, toonAsset(from, path));
}
