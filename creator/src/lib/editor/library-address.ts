// A file's address in the Library's answer is `<kind>:<path>` with the path the registry holds for the asset
// (server/docs/API.md "The extended Library"); the core's family names its slot default (`main`, a video's 720p). The two
// meet here, by path – no rule of either is copied.
import type { AssetFamily } from '@scutoons/media';

/** the path of a file address (`video:a/b.mp4` → `a/b.mp4`) */
export const pathOf = (address: string): string => address.slice(address.indexOf(':') + 1);

/** every file of every family, by its path: any of them finds its family */
export function familiesByPath(families: readonly AssetFamily[]): Map<string, AssetFamily> {
	const out = new Map<string, AssetFamily>();
	for (const family of families) {
		for (const v of family.versions) if (!out.has(v.path)) out.set(v.path, family);
	}
	return out;
}

/** a family's address as the server names it: the registered file of the asset that holds its main file, else its main */
export function familyAddress(family: Pick<AssetFamily, 'kind' | 'main' | 'assets' | 'versions'>, registered: ReadonlyMap<string, string>): string {
	const asset = family.versions.find((v) => v.path === family.main)?.asset ?? family.assets[0];
	return `${family.kind}:${(asset !== undefined ? registered.get(asset) : undefined) ?? family.main}`;
}
