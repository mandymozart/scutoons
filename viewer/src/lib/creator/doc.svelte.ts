// One config file of a toon, edited in all locales at once (chapter1.json in de + en, chapter2/scene-14.json, …).
// Same structure as the files the reader loads: text stays inline per locale; geometry and other shared fields are
// edited once and written to every locale that has not overridden them (see `setShared`).
import { ApiError } from '$lib/api/client.ts';
import {
	getDraft,
	getStatic,
	publishVersion,
	putDraft,
	type ContentVersion
} from '$lib/api/content.service.ts';
import { clone, deepEqual, getAt, setAt, type Path } from '@scutoons/content';

export type DocKind = 'chapter' | 'group' | 'season' | 'toon';
export type DocSource = 'store' | 'static' | 'new' | 'missing';

export const LOCALES = ['de', 'en'] as const;
export const SOURCE_LOCALE = 'de';

const HISTORY_LIMIT = 150;
const COALESCE_MS = 1200;
const AUTOSAVE_PREFIX = 'scutoons_creator:';

export interface AutosaveEntry {
	at: number;
	data: Record<string, any>;
	base: Record<string, number | null>;
}

export class SaveConflict extends Error {
	locale: string;
	latest: ContentVersion | null;
	constructor(locale: string, latest: ContentVersion | null) {
		super(`${locale}: someone saved a newer version`);
		this.locale = locale;
		this.latest = latest;
	}
}

export function kindOfPath(path: string): DocKind {
	if (/^chapter\d+\.json$/.test(path)) return 'chapter';
	if (/^season\d+\.json$/.test(path)) return 'season';
	if (path === 'toon.json') return 'toon';
	return 'group';
}

export class ContentDoc {
	readonly toon: string;
	readonly path: string;
	readonly kind: DocKind;
	readonly locales: string[];
	readonly primary: string;

	/** the edited files, per locale (null = this locale has no file) */
	data = $state<Record<string, any>>({});
	/** store version each locale is based on (null = not in the store yet) */
	base = $state<Record<string, number | null>>({});
	published = $state<Record<string, number | null>>({});
	source = $state<Record<string, DocSource>>({});
	/** canonical JSON of the last loaded/saved state, per locale */
	saved = $state<Record<string, string>>({});
	loading = $state(true);
	saving = $state(false);
	error = $state<string | null>(null);
	/** set when a local autosave newer than the loaded state exists */
	recoverable = $state<AutosaveEntry | null>(null);
	lastSavedAt = $state<number | null>(null);

	private undoStack = $state<string[]>([]);
	private redoStack = $state<string[]>([]);
	private lastKey: string | null = null;
	private lastTime = 0;

	constructor(toon: string, path: string, locales: readonly string[] = LOCALES, primary = SOURCE_LOCALE) {
		this.toon = toon;
		this.path = path;
		this.kind = kindOfPath(path);
		this.locales = [...locales];
		this.primary = primary;
	}

	get canUndo() {
		return this.undoStack.length > 0;
	}
	get canRedo() {
		return this.redoStack.length > 0;
	}

	/** locales whose data differs from what is stored */
	get dirtyLocales(): string[] {
		return this.locales.filter((l) => {
			const value = this.data[l];
			if (value === null || value === undefined) return false;
			return JSON.stringify(value) !== this.saved[l];
		});
	}
	get dirty() {
		return this.dirtyLocales.length > 0;
	}
	/** locales that exist in the content store */
	get stored(): string[] {
		return this.locales.filter((l) => this.base[l] !== null && this.base[l] !== undefined);
	}
	/** a stored locale whose newest version is not the published one */
	get unpublished(): string[] {
		return this.stored.filter((l) => this.published[l] !== this.base[l]);
	}

	// ── Loading ─────────────────────────────────────────────────────────────────────────────────────────

	async load() {
		this.loading = true;
		this.error = null;
		const data: Record<string, any> = {};
		const base: Record<string, number | null> = {};
		const published: Record<string, number | null> = {};
		const source: Record<string, DocSource> = {};
		try {
			await Promise.all(
				this.locales.map(async (locale) => {
					try {
						const wrapped = await getDraft(this.toon, locale, this.path);
						data[locale] = wrapped.content;
						base[locale] = wrapped.version.id;
						published[locale] = wrapped.published?.id ?? null;
						source[locale] = 'store';
						return;
					} catch (e) {
						if (!(e instanceof ApiError) || e.status !== 404) throw e;
					}
					const fallback = await getStatic(this.toon, locale, this.path);
					data[locale] = fallback;
					base[locale] = null;
					published[locale] = null;
					source[locale] = fallback === null ? 'missing' : 'static';
				})
			);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.loading = false;
			return;
		}
		this.adopt(data, base, published, source);
		this.checkAutosave();
		this.loading = false;
	}

	/** Starts a new file (e.g. a new chapter) in every locale. */
	create(make: (locale: string) => any) {
		const data: Record<string, any> = {};
		const base: Record<string, number | null> = {};
		const source: Record<string, DocSource> = {};
		for (const l of this.locales) {
			data[l] = make(l);
			base[l] = null;
			source[l] = 'new';
		}
		this.adopt(data, base, { ...base }, source);
		// a new file is unsaved: nothing stored yet
		for (const l of this.locales) this.saved[l] = '';
		this.loading = false;
	}

	private adopt(
		data: Record<string, any>,
		base: Record<string, number | null>,
		published: Record<string, number | null>,
		source: Record<string, DocSource>
	) {
		this.data = data;
		this.base = base;
		this.published = published;
		this.source = source;
		const saved: Record<string, string> = {};
		for (const l of this.locales) saved[l] = data[l] === null || data[l] === undefined ? '' : JSON.stringify(data[l]);
		this.saved = saved;
		this.undoStack = [];
		this.redoStack = [];
		this.lastKey = null;
	}

	// ── History ─────────────────────────────────────────────────────────────────────────────────────────

	private snapshot() {
		return JSON.stringify(this.data);
	}

	/** Records the current state for undo. Calls with the same `key` within ~1s are merged (typing, dragging). */
	checkpoint(key?: string) {
		const now = Date.now();
		if (key && key === this.lastKey && now - this.lastTime < COALESCE_MS) {
			this.lastTime = now;
			return;
		}
		this.undoStack.push(this.snapshot());
		if (this.undoStack.length > HISTORY_LIMIT) this.undoStack.shift();
		this.redoStack = [];
		this.lastKey = key ?? null;
		this.lastTime = now;
	}

	undo() {
		const previous = this.undoStack.pop();
		if (previous === undefined) return;
		this.redoStack.push(this.snapshot());
		this.data = JSON.parse(previous);
		this.lastKey = null;
	}

	redo() {
		const next = this.redoStack.pop();
		if (next === undefined) return;
		this.undoStack.push(this.snapshot());
		this.data = JSON.parse(next);
		this.lastKey = null;
	}

	// ── Editing ─────────────────────────────────────────────────────────────────────────────────────────

	/** locales that have a file */
	get present(): string[] {
		return this.locales.filter((l) => this.data[l] !== null && this.data[l] !== undefined);
	}

	get(locale: string, path: Path) {
		return getAt(this.data[locale], path);
	}

	/**
	 * A shared field (geometry, style, image …): written to the source locale and to every locale whose value
	 * still equals the source's old value. Locales that differ keep their value (a per-locale override).
	 */
	setShared(path: Path, value: any, key?: string) {
		this.checkpoint(key ?? path.join('.'));
		const old = clone(getAt(this.data[this.primary], path));
		for (const l of this.present) {
			if (l === this.primary || deepEqual(getAt(this.data[l], path), old)) {
				setAt(this.data[l], path, clone(value));
			}
		}
	}

	/** A per-locale field (text) or an explicit override. */
	setLocal(locale: string, path: Path, value: any, key?: string) {
		if (!this.data[locale]) return;
		this.checkpoint(key ?? `${locale}:${path.join('.')}`);
		setAt(this.data[locale], path, clone(value));
	}

	/** Locales whose value at `path` differs from the source locale (overrides). */
	overrides(path: Path): string[] {
		const reference = getAt(this.data[this.primary], path);
		return this.present.filter((l) => l !== this.primary && !deepEqual(getAt(this.data[l], path), reference));
	}

	resetOverride(locale: string, path: Path) {
		this.setLocal(locale, path, getAt(this.data[this.primary], path));
	}

	/** A structural change applied to every locale file (insert/remove/reorder slices, dialogs, events …). */
	structural(change: (file: any, locale: string) => void, key?: string) {
		this.checkpoint(key);
		for (const l of this.present) change(this.data[l], l);
	}

	/** Copies the source locale file to a locale that has none. */
	createLocale(locale: string, transform?: (file: any) => any) {
		this.checkpoint();
		const copy = clone(this.data[this.primary]);
		this.data[locale] = transform ? transform(copy) : copy;
		if (this.source[locale] === 'missing') this.source[locale] = 'new';
	}

	// ── Store ───────────────────────────────────────────────────────────────────────────────────────────

	/** Saves every changed locale as a new draft version. `force` ignores conflicts (overwrites). */
	async save(message?: string, force = false): Promise<string[]> {
		const locales = this.dirtyLocales;
		if (!locales.length) return [];
		this.saving = true;
		this.error = null;
		const done: string[] = [];
		try {
			for (const l of locales) {
				const content = $state.snapshot(this.data[l]);
				try {
					const result = await putDraft(this.toon, l, this.path, content, force ? undefined : this.base[l], message);
					this.base[l] = result.version.id;
					this.source[l] = 'store';
					this.saved[l] = JSON.stringify(content);
					done.push(l);
				} catch (e) {
					if (e instanceof ApiError && e.status === 409) throw new SaveConflict(l, e.body?.latest ?? null);
					throw e;
				}
			}
			this.lastSavedAt = Date.now();
			if (!this.dirty) this.clearAutosave();
			return done;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			throw e;
		} finally {
			this.saving = false;
		}
	}

	/** Publishes the newest stored version of every locale (save first). */
	async publish(): Promise<string[]> {
		const done: string[] = [];
		this.saving = true;
		try {
			for (const l of this.stored) {
				const result = await publishVersion(this.toon, l, this.path, this.base[l] ?? undefined);
				this.published[l] = result.version.id;
				done.push(l);
			}
			return done;
		} finally {
			this.saving = false;
		}
	}

	// ── Local autosave (browser only, per file) ─────────────────────────────────────────────────────────

	get autosaveKey() {
		return `${AUTOSAVE_PREFIX}${this.toon}:${this.path}`;
	}

	autosave() {
		try {
			if (!this.dirty) {
				localStorage.removeItem(this.autosaveKey);
				return;
			}
			const entry: AutosaveEntry = { at: Date.now(), data: $state.snapshot(this.data), base: $state.snapshot(this.base) };
			localStorage.setItem(this.autosaveKey, JSON.stringify(entry));
		} catch {
			/* storage full / unavailable – the store is the real save */
		}
	}

	clearAutosave() {
		try {
			localStorage.removeItem(this.autosaveKey);
		} catch {
			/* ignore */
		}
		this.recoverable = null;
	}

	private checkAutosave() {
		try {
			const raw = localStorage.getItem(this.autosaveKey);
			if (!raw) return;
			const entry = JSON.parse(raw) as AutosaveEntry;
			if (!entry?.data || deepEqual(entry.data, $state.snapshot(this.data))) {
				localStorage.removeItem(this.autosaveKey);
				return;
			}
			this.recoverable = entry;
		} catch {
			/* ignore */
		}
	}

	/** Loads the local autosave into the editor (undoable); the store base stays as loaded. */
	recover() {
		if (!this.recoverable) return;
		this.checkpoint();
		this.data = clone(this.recoverable.data);
		this.recoverable = null;
	}

	/** true when the autosave was made on top of an older store version than the loaded one */
	get recoverableIsStale(): boolean {
		const entry = this.recoverable;
		if (!entry) return false;
		return this.locales.some((l) => (entry.base?.[l] ?? null) !== (this.base[l] ?? null));
	}
}
