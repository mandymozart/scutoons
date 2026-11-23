// The creator's Experiences workspace, first slice: the list's rows and an experience's facts and outline, read from the
// catalog document `experience:<id>` (@scutoons/content xr.ts). The configuration is OSCT's and only roughly checked by
// the server, so everything here reads it defensively; whether it plays is the engine's guard in the reader (RULES A2).
import type { CatalogDoc } from '@scutoons/api';
import type { XrOsctSource } from '@scutoons/content';

export type ExperienceState = 'published' | 'draft' | 'unpublished';

export interface ExperienceRow {
	id: string;
	key: string;
	title: Record<string, string>;
	type: string;
	toons: string[];
	state: ExperienceState;
	/** the newest version's id (what Publish publishes) */
	latest: number | null;
}

type Obj = Record<string, unknown>;
const obj = (v: unknown): Obj | null => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Obj) : null);
const list = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);
const str = (v: unknown): string => (typeof v === 'string' ? v : '');
const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);

/** a locale map's own strings (anything else dropped) */
export function localeMap(v: unknown): Record<string, string> {
	const o = obj(v);
	return o ? Object.fromEntries(Object.entries(o).filter((e): e is [string, string] => typeof e[1] === 'string' && e[1].trim() !== '')) : {};
}

/** the title in the creator's language, else the first one, else the id */
export function titleIn(title: Record<string, string>, locale: string, id: string): string {
	return title[locale] ?? Object.values(title)[0] ?? id;
}

export function experienceState(e: Pick<CatalogDoc, 'latest' | 'published' | 'draft'>): ExperienceState {
	if (e.published === null) return e.latest === null ? 'published' : 'unpublished';
	return e.draft ? 'draft' : 'published';
}

/** the list: every experience not removed, by its title in `locale` */
export function experienceRows(entries: CatalogDoc[], locale: string): ExperienceRow[] {
	return entries
		.filter((e) => !e.removed)
		.map((e) => {
			const c = obj(e.content) ?? {};
			return { id: e.id, key: e.key, title: localeMap(c.title), type: str(c.type), toons: list(c.toons).map(str).filter(Boolean), state: experienceState(e), latest: e.latest };
		})
		.sort((a, b) => titleIn(a.title, locale, a.id).localeCompare(titleIn(b.title, locale, b.id)));
}

export interface ExperienceFacts {
	id: string;
	type: string;
	title: Record<string, string>;
	toons: string[];
	source: XrOsctSource | null;
	/** the CDN-root folder its files lie below */
	assets: string;
}

export function experienceFacts(content: unknown, id: string): ExperienceFacts {
	const c = obj(content) ?? {};
	const osct = obj(obj(c.source)?.osct);
	const source = osct ? { experience: str(osct.experience), ...(str(osct.scene) ? { scene: str(osct.scene) } : {}), build: str(osct.build), schema: str(osct.schema), app: str(osct.app) } : null;
	return { id: str(c.id) || id, type: str(c.type), title: localeMap(c.title), toons: list(c.toons).map(str).filter(Boolean), source, assets: str(c.assets).replace(/^\/+|\/+$/g, '') };
}

/**
 * a configuration path (`/assets/content/targets/a.jpg`) as a CDN-root path below the experience's folder; an absolute
 * URL stays as it is
 */
export function contentUrl(assets: string, src: string, cdn: (path: string) => string): string | null {
	if (!src) return null;
	if (/^https?:\/\//.test(src)) return src;
	const path = src.replace(/^\/+/, '');
	return cdn(assets ? `${assets}/${path}` : path);
}

export interface OutlineTarget {
	id: string;
	/** the configuration's path of the printed picture */
	image: string;
	/** a shared entity it shows (`ref`), else null */
	entity: string | null;
	enabled: boolean;
}
export interface OutlineEntry {
	id: string;
	title: string;
	category: string;
	target: OutlineTarget | null;
}
export interface OutlineSurface {
	id: string;
	label: string;
	widthMm: number | null;
	heightMm: number | null;
	entries: OutlineEntry[];
}
export interface OutlineScene {
	id: string;
	label: string;
	surfaces: OutlineSurface[];
	/** entries naming a surface the scene does not have */
	loose: OutlineEntry[];
}
export interface Outline {
	scenes: OutlineScene[];
	/** entries naming a scene the configuration does not have */
	orphans: OutlineEntry[];
	/** every entry */
	entries: number;
}

function outlineEntry(e: Obj): OutlineEntry {
	const t = obj(e.target);
	const target = t ? { id: str(t.id) || str(e.id), image: str(t.imageSrc), entity: str(obj(t.entity)?.ref) || null, enabled: t.enabled !== false } : null;
	return { id: str(e.id), title: str(e.title), category: str(e.category), target };
}

/** scenes › surfaces › entries; an entry without a surface sits on its scene's only one */
export function experienceOutline(configuration: unknown): Outline {
	const c = obj(configuration) ?? {};
	const entries = list(c.entries).map(obj).filter((e): e is Obj => !!e);
	const used = new Set<Obj>();
	const scenes = list(c.scenes)
		.map(obj)
		.filter((s): s is Obj => !!s)
		.map((s): OutlineScene => {
			const id = str(s.id);
			const own = entries.filter((e) => str(e.scene) === id);
			own.forEach((e) => used.add(e));
			const raw = list(s.surfaces).map(obj).filter((x): x is Obj => !!x);
			const only = raw.length === 1 ? str(raw[0].id) : '';
			const on = (e: Obj) => str(e.surface) || only;
			const surfaces = raw.map((x) => ({ id: str(x.id), label: str(x.label), widthMm: num(x.widthMm), heightMm: num(x.heightMm), entries: own.filter((e) => on(e) === str(x.id)).map(outlineEntry) }));
			const ids = new Set(surfaces.map((x) => x.id));
			return { id, label: str(s.label), surfaces, loose: own.filter((e) => !ids.has(on(e))).map(outlineEntry) };
		});
	return { scenes, orphans: entries.filter((e) => !used.has(e)).map(outlineEntry), entries: entries.length };
}

export interface EntityCount {
	id: string;
	nodes: number;
	/** node type → count, most first */
	types: { type: string; count: number }[];
}

/** the shared entities with their nodes (the root and every child) counted by type */
export function entityCounts(configuration: unknown): EntityCount[] {
	const shared = obj(obj(configuration)?.entities) ?? {};
	return Object.entries(shared).map(([id, root]) => {
		const counts = new Map<string, number>();
		const walk = (node: unknown, depth: number) => {
			const n = obj(node);
			if (!n || depth > 64) return;
			const type = str(n.type) || '?';
			counts.set(type, (counts.get(type) ?? 0) + 1);
			list(n.children).forEach((child) => walk(child, depth + 1));
		};
		walk(root, 0);
		const types = [...counts].map(([type, count]) => ({ type, count })).sort((a, b) => b.count - a.count || a.type.localeCompare(b.type));
		return { id, nodes: types.reduce((sum, t) => sum + t.count, 0), types };
	});
}

export interface PluginRow {
	id: string;
	api: string;
	/** the code's size in bytes (UTF-8) */
	bytes: number;
}

export function pluginRows(plugins: unknown): PluginRow[] {
	const encoder = new TextEncoder();
	return list(plugins)
		.map(obj)
		.filter((p): p is Obj => !!p)
		.map((p) => ({ id: str(p.id), api: str(p.api), bytes: encoder.encode(str(p.code)).length }));
}

/** what an experience's tab lists from its document: the outline, the shared entities, the plugins */
export function experienceContents(content: unknown): { outline: Outline; entities: EntityCount[]; plugins: PluginRow[] } {
	const c = obj(content) ?? {};
	return { outline: experienceOutline(c.configuration), entities: entityCounts(c.configuration), plugins: pluginRows(c.plugins) };
}

/** a size in KB for people (at least 1 when there is anything) */
export const kb = (bytes: number) => (bytes > 0 ? Math.max(1, Math.round(bytes / 1024)) : 0);

/** a surface's size in words: `420 × 594 mm` */
export const surfaceSize = (s: Pick<OutlineSurface, 'widthMm' | 'heightMm'>) => (s.widthMm !== null && s.heightMm !== null ? `${s.widthMm} × ${s.heightMm} mm` : '');

export const experienceKey = (id: string) => `experience:${id}`;

/** the experience's page in the creator */
export const experienceHref = (id: string) => `/experiences/${encodeURIComponent(id)}`;

/** the reader's preview of the newest draft */
export const readerHref = (viewer: string, id: string) => `${viewer}/xr/${encodeURIComponent(id)}?draft=1`;
