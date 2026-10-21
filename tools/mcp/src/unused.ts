// The admin's Assets → Unused list for the AI (issue 152): what a cleanup could delete, read only. The server decides
// what is unused (UnusedList, the usage index); this only folds its rows into groups – a map tile set as one group, every
// other file by its folder – with totals, and lists the files themselves when asked. No delete tool: the cleanup stays
// in the admin (owner 2026-10-05, CUTOVER 10.28).
import type { AssetUnused, AssetUnusedList, AssetUnusedTotals } from '@scutoons/api';

export interface UnusedGroup {
	/** a tile set's folder, else the files' folder */
	group: string;
	tiles: boolean;
	files: number;
	bytes: number;
	/** what the files are: `current` unreferenced files of the current version, `old` left by a rename, `archive` old bytes */
	what: Partial<Record<'current' | 'old' | 'archive', number>>;
	/** files of assets that are wholly unused (the cleanup deletes the asset) vs. leftovers of a used asset */
	whole_assets: number;
}

export interface UnusedFile {
	path: string;
	size: number | null;
	what: 'current' | 'old' | 'archive';
	group: string;
	whole_asset: boolean;
}

export interface UnusedSummary {
	totals: AssetUnusedTotals;
	matching: AssetUnusedTotals;
	groups: UnusedGroup[];
	files?: { total: number; offset: number; items: UnusedFile[]; more: boolean };
}

const folder = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')));

/** a row's group: the tile set's folder (`tiles:<set>` rows), else each file's own folder */
const groupOf = (row: AssetUnused, path: string) => (row.tiles !== null ? row.slug : folder(path));

/**
 * The rows (every file – the list read with `all: 1`) as groups, biggest first; `files` also lists the files (sorted by
 * path, `offset` / `limit` – a tile set has thousands)
 */
export function summarizeUnused(list: AssetUnusedList, options: { files?: boolean; offset?: number; limit?: number } = {}): UnusedSummary {
	const groups = new Map<string, UnusedGroup>();
	const files: UnusedFile[] = [];
	for (const row of list.items) {
		for (const f of row.files) {
			const name = groupOf(row, f.path);
			let g = groups.get(name);
			if (!g) groups.set(name, (g = { group: name, tiles: row.tiles !== null, files: 0, bytes: 0, what: {}, whole_assets: 0 }));
			g.files++;
			g.bytes += f.size ?? 0;
			g.what[f.what] = (g.what[f.what] ?? 0) + 1;
			if (row.all_unused) g.whole_assets++;
			if (options.files) files.push({ path: f.path, size: f.size, what: f.what, group: name, whole_asset: row.all_unused });
		}
	}
	const out: UnusedSummary = {
		totals: list.totals,
		matching: list.matching,
		groups: [...groups.values()].sort((a, b) => b.bytes - a.bytes || a.group.localeCompare(b.group))
	};
	if (options.files) {
		const offset = options.offset ?? 0;
		const limit = options.limit ?? 500;
		files.sort((a, b) => a.path.localeCompare(b.path));
		out.files = { total: files.length, offset, items: files.slice(offset, offset + limit), more: offset + limit < files.length };
	}
	return out;
}
