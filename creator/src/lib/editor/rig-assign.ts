// The Assignment tab in the creator (#283): a character's rig `rig:character.<id>` as a draft document (the catalog
// documents' own plumbing – GET /entities/<key>?draft=1, PUT a new draft version, publish; the API checks a rig with the
// core on publish, #313) and the catalog lists the form picks from (GET /catalog/docs/instruments | presets | rigs). The
// rules – which characters lack a default rig, which documents a rig's bundle holds – are the core's (space-web
// `rig_coverage`, `rig_catalog_bundle`): this only builds their requests from what the API listed.
import type { CatalogInstrument, CatalogPreset, RigDoc } from '@scutoons/rig-designer';
import { ApiError, type ContentApi } from '@scutoons/api';

/** a catalog list's entry as GET /catalog/docs/<kind> sends it (the fields used here) */
export interface CatalogEntryLike {
	id: string;
	live: boolean;
	draft: boolean;
	removed: boolean;
	content: unknown;
}

/** what the form and the core's requests read: the documents, the pickers' lists, the rigs' states */
export interface RigLists {
	instruments: CatalogInstrument[];
	presets: CatalogPreset[];
	/** the documents as stored (the newest version each), for the bundle */
	instrumentDocs: unknown[];
	presetDocs: unknown[];
	rigs: { id: string; live: boolean; draft: boolean; removed: boolean }[];
}

export const NO_RIG_LISTS: RigLists = { instruments: [], presets: [], instrumentDocs: [], presetDocs: [], rigs: [] };

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const text = (v: unknown) => (isObject(v) ? (Object.fromEntries(Object.entries(v).filter(([, s]) => typeof s === 'string')) as Record<string, string>) : undefined);

/** the three catalog lists as the form reads them; removed entries are gone, the pickers sorted by id */
export function rigLists(instruments: CatalogEntryLike[], presets: CatalogEntryLike[], rigs: CatalogEntryLike[]): RigLists {
	const keep = (list: CatalogEntryLike[]) => list.filter((e) => !e.removed && isObject(e.content));
	const byId = <T extends { id: string }>(a: T, b: T) => a.id.localeCompare(b.id);
	const i = keep(instruments);
	const p = keep(presets);
	return {
		instruments: i.map((e) => ({ id: e.id, kind: (e.content as { kind?: string }).kind, name: text((e.content as { name?: unknown }).name) })).sort(byId),
		presets: p.map((e) => ({ id: e.id, instrument: String((e.content as { instrument?: unknown }).instrument ?? ''), name: text((e.content as { name?: unknown }).name) })).sort(byId),
		instrumentDocs: i.map((e) => e.content),
		presetDocs: p.map((e) => e.content),
		rigs: rigs.map(({ id, live, draft, removed }) => ({ id, live, draft, removed }))
	};
}

/** the core's coverage request (space-web `rig_coverage`): the registry's characters with their personas, the rigs' states */
export function coverageRequest(characters: { id: string; personas?: { id: string }[] }[], lists: RigLists): string {
	return JSON.stringify({ characters: characters.map((c) => ({ id: c.id, personas: (c.personas ?? []).map((p) => p.id) })), rigs: lists.rigs });
}

/** the core's bundle request (space-web `rig_catalog_bundle`): the rig with the catalog's documents */
export function bundleRequest(rig: RigDoc, lists: RigLists): string {
	return JSON.stringify({ rig, instruments: lists.instrumentDocs, presets: lists.presetDocs });
}

/** the creator's draft calls (`$lib/api.ts`); tests pass fakes */
export type Drafts = Pick<ContentApi, 'getDraft' | 'putDraft' | 'publishVersion'>;
export type RigResult = { ok: true } | { ok: false; message: string };

export const rigKey = (subject: string) => `rig:${subject}`;

const failed = (e: unknown): { ok: false; message: string } => ({ ok: false, message: e instanceof Error ? e.message : String(e) });

/** the draft store of `rig:<subject>`: it holds the version the next save builds on (a conflict answers 409, never a silent overwrite) */
export function createRigStore(drafts: Drafts) {
	const base = new Map<string, number | null>();
	return {
		/** the newest version (a draft, else the published one); null = none stored yet */
		async load(subject: string): Promise<{ ok: true; rig: RigDoc | null } | { ok: false; message: string }> {
			try {
				const wrapped = await drafts.getDraft(rigKey(subject));
				base.set(subject, wrapped.version.id);
				return { ok: true, rig: isObject(wrapped.content) && Array.isArray(wrapped.content.track) ? (wrapped.content as unknown as RigDoc) : null };
			} catch (e) {
				if (e instanceof ApiError && e.status === 404) {
					base.set(subject, null);
					return { ok: true, rig: null };
				}
				return failed(e);
			}
		},
		async save(subject: string, rig: RigDoc): Promise<RigResult> {
			try {
				const r = await drafts.putDraft(rigKey(subject), rig, base.get(subject) ?? null, 'assignment');
				base.set(subject, r.version.id);
				return { ok: true };
			} catch (e) {
				return failed(e);
			}
		},
		/** publishes the saved draft (the API asks the core first, #313) */
		async publish(subject: string): Promise<RigResult> {
			const version = base.get(subject);
			if (version == null) return { ok: false, message: rigKey(subject) };
			try {
				await drafts.publishVersion(rigKey(subject), version);
				return { ok: true };
			} catch (e) {
				return failed(e);
			}
		}
	};
}
