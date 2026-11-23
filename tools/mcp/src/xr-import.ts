// What an OSCT content version becomes as a SCUTOONS XR experience (research/XR.md §10.8): the game configuration as
// OSCT wrote it, cut down to one scene when asked, its files named by content paths (`/assets/content/…`, which the
// engine resolves against the experience's own folder) and its plugins taken out (the document carries their code).
// Pure: the tool (xr.ts) fetches, uploads and saves.

type Row = Record<string, unknown>;

const CONTENT = '/assets/content/';
const MEDIA = /\.(glb|gltf|webp|png|jpe?g|avif|gif|svg|mp4|webm|mp3|m4a|ogg|wav|flac|vtt|hdr|ktx2)$/i;

export interface XrImportPlan {
	/** the configuration for the document: without `plugins`, every file named by its content path */
	configuration: Row;
	/** the files it uses: where each is now, and its content path (without the leading slash) */
	files: Array<{ from: string; path: string }>;
	/** the plugins to carry as code: where each compiled module is now */
	plugins: Array<{ id: string; api: string; from: string }>;
	/** plugins left out: built for an older plugin API than the version's newest */
	skipped: string[];
}

const isRow = (value: unknown): value is Row => typeof value === 'object' && value !== null && !Array.isArray(value);
const rows = (value: unknown): Row[] => (Array.isArray(value) ? value.filter(isRow) : []);
const major = (api: unknown): number => (typeof api === 'string' ? Number.parseInt(api, 10) : Number.NaN);

/** a file of the content: a content path, or a media file by its full address (the Library's) */
function isFile(value: string): boolean {
	if (value.startsWith(CONTENT)) return true;
	if (!/^https?:\/\//i.test(value)) return false;
	try {
		return MEDIA.test(new URL(value).pathname);
	} catch {
		return false;
	}
}

/** every shared entity a value refers to (`{ ref }`), and what those refer to */
function usedEntities(from: unknown, entities: Row): Set<string> {
	const used = new Set<string>();
	const walk = (value: unknown): void => {
		if (Array.isArray(value)) return value.forEach(walk);
		if (!isRow(value)) return;
		const ref = value.ref;
		if (typeof ref === 'string' && Object.hasOwn(entities, ref) && !used.has(ref)) {
			used.add(ref);
			walk(entities[ref]);
		}
		Object.values(value).forEach(walk);
	};
	walk(from);
	return used;
}

/**
 * `versionUrl`: the version's folder (`<content base>/<build>`), where its content paths lie. `scene`: only this
 * scene with its entries and the shared entities they use; without it the whole experience.
 */
export function planImport(raw: unknown, versionUrl: string, scene?: string): XrImportPlan {
	if (!isRow(raw) || !Array.isArray(raw.scenes) || !Array.isArray(raw.entries)) throw new Error('this is no game configuration (no scenes or entries)');
	const config = structuredClone(raw);
	const base = versionUrl.replace(/\/+$/, '');

	if (scene) {
		const ids = rows(config.scenes).map((s) => String(s.id));
		if (!ids.includes(scene)) throw new Error(`the version has no scene "${scene}" (it has: ${ids.join(', ')})`);
		config.scenes = rows(config.scenes).filter((s) => s.id === scene);
		config.initialScene = scene;
		config.entries = rows(config.entries).filter((e) => e.scene === scene);
		if (isRow(config.entities)) {
			const all = config.entities;
			const used = usedEntities(config.entries, all);
			config.entities = Object.fromEntries(Object.entries(all).filter(([ref]) => used.has(ref)));
		}
	}

	const declared = rows(config.plugins).filter((p) => typeof p.id === 'string' && typeof p.src === 'string' && !Number.isNaN(major(p.api)));
	const newest = Math.max(...declared.map((p) => major(p.api)));
	const plugins = declared
		.filter((p) => major(p.api) === newest)
		.map((p) => ({ id: String(p.id), api: String(p.api), from: /^https?:\/\//i.test(String(p.src)) ? String(p.src) : base + String(p.src) }));
	const skipped = declared.filter((p) => major(p.api) !== newest).map((p) => `${p.id} (plugin API ${p.api})`);
	delete config.plugins;

	const paths = new Map<string, string>();
	const taken = new Set<string>();
	const pathOf = (source: string): string => {
		const known = paths.get(source);
		if (known) return known;
		let path: string;
		if (source.startsWith(CONTENT)) path = source.split(/[?#]/)[0].slice(1);
		else {
			const parts = new URL(source).pathname.split('/').filter(Boolean).slice(-2);
			path = `assets/content/library/${parts.join('-').replace(/[^A-Za-z0-9_.-]/g, '_')}`;
		}
		const dot = path.lastIndexOf('.');
		const [stem, extension] = [path.slice(0, dot), path.slice(dot)];
		for (let n = 2; taken.has(path); n++) path = `${stem}-${n}${extension}`;
		taken.add(path);
		paths.set(source, path);
		return path;
	};
	const named = (value: unknown): unknown => {
		if (typeof value === 'string') return isFile(value) ? `/${pathOf(value)}` : value;
		if (Array.isArray(value)) return value.map(named);
		if (isRow(value)) return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, named(child)]));
		return value;
	};

	// subtitles are listed beside the scenes: only those of a video that is still in
	const { subtitles, ...rest } = config;
	const configuration = named(rest) as Row;
	if (Array.isArray(subtitles)) configuration.subtitles = named(subtitles.filter((s) => isRow(s) && typeof s.media === 'string' && paths.has(s.media)));

	const files = [...paths].map(([source, path]) => ({ from: source.startsWith(CONTENT) ? base + source : source, path }));
	return { configuration, files, plugins, skipped };
}
