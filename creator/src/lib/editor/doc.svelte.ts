// One entity of a toon (agents/DATA.md: `chapter:catpope:1`, `scene:catpope:2:scene-14`, `season:catpope:1`,
// `toon-settings:catpope`, `characters` …), edited in all locales at once. Localized entities are stored per locale
// (`<entity>:<locale>`); language-neutral ones under the entity itself (locale `_` in the editor).
//
// Chapters, scenes and seasons are ONE language-neutral document each (agents/NEUTRAL.md): structure once, every text a
// locale map `{ de: …, en: … }` (`texts` = the toon's languages). `edit` / `set` change the document, `text` / `setText`
// one language of a text leaf (the creator header's text language). An entity without a neutral document yet
// (production before its conversion) is loaded from its per-locale documents and converted in memory (`neutralizeDocs`,
// `converted` = the report): saving stores the neutral document; the per-locale keys are never written again.
//
// Other localized entities (toon settings, script) keep one document per locale: `setShared` a shared field in every
// locale that has not overridden it, `setLocal` one locale, `createLocale` a missing locale from the source. Language-
// neutral entities (cast, credits, the map …) are one document under locale `_`, edited with `edit` / `set`.
// The locales and the source ("primary") come from the toon (`ToonIndex.languages` / `.sourceLanguage`, loaded with
// `loadToonLanguages` in ./languages.ts), never from constants.
import { ApiError } from '$lib/api.ts';
import { getDraft, publishVersion, putDraft, type ContentVersion } from '$lib/api.ts';
import {
	clone,
	deepEqual,
	entityKeyFor,
	getAt,
	isLocaleValue,
	neutralizeDocs,
	parseEntityKey,
	resolveGroup,
	resolveSlice,
	setAt,
	type Group,
	type NeutralDifference,
	type NeutralKind,
	type Path,
	type Slice
} from '@scutoons/content';
import { repairBooleans } from '$lib/editor/repair-booleans.ts';

export type DocKind = 'chapter' | 'group' | 'season' | 'toon' | 'index' | 'credits' | 'people' | 'roles' | 'characters' | 'cast' | 'moods' | 'requests' | 'map' | 'events' | 'script';
export type DocSource = 'store' | 'new' | 'missing';

const HISTORY_LIMIT = 150;
const COALESCE_MS = 1200;
const AUTOSAVE_PREFIX = 'scutoons_creator:';

export interface AutosaveEntry {
	at: number;
	data: Record<string, any>;
	base: Record<string, number | null>;
}

/** a refused save of one locale (e.g. 422 invalid_content): the message names the locale's problem */
export class SaveError extends Error {
	locale: string;
	constructor(locale: string, cause: unknown) {
		super(cause instanceof Error ? cause.message : String(cause), { cause });
		this.locale = locale;
	}
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

/** The text languages of a language-neutral chapter / scene / season document (its locale maps), source first */
export interface DocTexts {
	source: string;
	languages: readonly string[];
}

/** A neutral document converted in memory from the per-locale ones (saving stores it) */
export interface DocConversion {
	/** what the source locale overrode (@scutoons/content neutralize.ts) */
	report: NeutralDifference[];
	/** the languages it was made of (source first) */
	locales: string[];
}

/** The editor kind of an entity (which validation and panels apply) */
export function kindOfEntity(entity: string): DocKind {
	switch (parseEntityKey(entity)?.kind) {
		case 'chapter':
			return 'chapter';
		case 'season':
			return 'season';
		case 'toon-settings':
			return 'toon';
		case 'toon':
			return 'index';
		case 'credits':
			return 'credits';
		case 'people':
			return 'people';
		case 'roles':
			return 'roles';
		case 'characters':
			return 'characters';
		case 'cast':
			return 'cast';
		case 'moods':
			return 'moods';
		case 'asset-requests':
			return 'requests';
		case 'locations':
		case 'routes':
		case 'map':
			return 'map';
		case 'events':
			return 'events';
		case 'script':
			return 'script';
		default:
			return 'group';
	}
}

export class ContentDoc {
	/** the entity (key without locale), e.g. `chapter:catpope:1` */
	readonly entity: string;
	/** the toon it belongs to ('' for platform-wide entities) */
	readonly toon: string;
	readonly kind: DocKind;
	readonly locales: string[];
	/** the toon's source language (the first of `locales`) */
	readonly primary: string;

	/** the edited documents, per locale (null = this locale has none) */
	data = $state<Record<string, any>>({});
	/** stored version each locale is based on (null = not stored yet) */
	base = $state<Record<string, number | null>>({});
	published = $state<Record<string, number | null>>({});
	/** when the version each locale is based on was saved (ISO; shown as a date, never the version id) */
	changedAt = $state<Record<string, string | null>>({});
	source = $state<Record<string, DocSource>>({});
	/** canonical JSON of the last loaded/saved state, per locale */
	saved = $state<Record<string, string>>({});
	loading = $state(true);
	saving = $state(false);
	error = $state<string | null>(null);
	/** set when a local autosave newer than the loaded state exists */
	recoverable = $state<AutosaveEntry | null>(null);
	/** set when the neutral document was converted from the per-locale ones on load (not stored yet) */
	converted = $state<DocConversion | null>(null);
	lastSavedAt = $state<number | null>(null);

	private undoStack = $state<string[]>([]);
	private redoStack = $state<string[]>([]);
	private lastKey: string | null = null;
	private lastTime = 0;
	/** time of the newest undo step (pages editing several docs undo the most recently edited one) */
	lastEditAt = $state(0);

	/** a language-neutral chapter / scene / season: the languages of its texts (locale maps); null for other documents */
	readonly texts: DocTexts | null;

	/**
	 * `locales`: the toon's languages (`['_']` for language-neutral entities); `primary`: its source language;
	 * `texts`: a neutral chapter / scene / season's text languages (languages.ts `toonDoc`).
	 */
	constructor(entity: string, locales: readonly string[], primary: string, texts: DocTexts | null = null) {
		this.entity = entity;
		this.toon = parseEntityKey(entity)?.toon ?? '';
		this.kind = kindOfEntity(entity);
		this.locales = [primary, ...locales.filter((l) => l !== primary)];
		this.primary = primary;
		this.texts = texts ? { source: texts.source, languages: [texts.source, ...texts.languages.filter((l) => l !== texts.source)] } : null;
	}

	/** converted from the per-locale documents on load and not stored yet (only a save of this document stores it) */
	get unconverted(): boolean {
		return !!this.converted && this.base[this.primary] == null;
	}

	/** a language-neutral document with locale-map texts (chapter, scene, season) */
	get neutral(): boolean {
		return this.texts !== null;
	}
	/** the languages of the texts: a neutral document's (the toon's), else the per-locale documents there are */
	get textLanguages(): string[] {
		return this.texts ? [...this.texts.languages] : this.present;
	}
	/** the source language of the texts */
	get textSource(): string {
		return this.texts?.source ?? this.primary;
	}

	/** The stored key of one locale (`chapter:catpope:1:de`; language-neutral entities: the entity) */
	key(locale: string): string {
		return entityKeyFor(this.entity, locale);
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
		let legacy = null as { doc: unknown; conversion: DocConversion } | null;
		const data: Record<string, any> = {};
		const base: Record<string, number | null> = {};
		const published: Record<string, number | null> = {};
		const source: Record<string, DocSource> = {};
		try {
			await Promise.all(
				this.locales.map(async (locale) => {
					try {
						const wrapped = await getDraft(this.key(locale));
						data[locale] = wrapped.content;
						base[locale] = wrapped.version.id;
						this.changedAt[locale] = wrapped.version.created_at ?? null;
						published[locale] = wrapped.published?.id ?? null;
						source[locale] = 'store';
						return;
					} catch (e) {
						if (!(e instanceof ApiError) || e.status !== 404) throw e;
					}
					// no neutral document yet: the per-locale ones, converted in memory
					legacy = this.texts ? await this.loadLegacy() : null;
					if (legacy) {
						data[locale] = legacy.doc;
						base[locale] = null;
						published[locale] = null;
						this.changedAt[locale] = null;
						source[locale] = 'new';
						return;
					}
					data[locale] = null;
					base[locale] = null;
					published[locale] = null;
					source[locale] = 'missing';
				})
			);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.loading = false;
			return;
		}
		this.adopt(data, base, published, source);
		// a conversion is not stored yet: unsaved until the next save stores the neutral document
		if (legacy) this.saved[this.primary] = '';
		this.converted = legacy?.conversion ?? null;
		this.checkAutosave();
		this.loading = false;
	}

	/**
	 * The newest per-locale documents of a neutral entity (`chapter:catpope:2:de` …) as one neutral document
	 * (@scutoons/content neutralizeDocs); null when there are none
	 */
	private async loadLegacy(): Promise<{ doc: unknown; conversion: DocConversion } | null> {
		const texts = this.texts!;
		const found: [string, unknown][] = [];
		await Promise.all(
			texts.languages.map(async (locale) => {
				try {
					const wrapped = await getDraft(`${this.entity}:${locale}`);
					found.push([locale, wrapped.content]);
				} catch (e) {
					if (!(e instanceof ApiError) || e.status !== 404) throw e;
				}
			})
		);
		if (!found.length) return null;
		found.sort((a, b) => texts.languages.indexOf(a[0]) - texts.languages.indexOf(b[0]));
		const kind = parseEntityKey(this.entity)!.kind as NeutralKind;
		const result = neutralizeDocs(kind, Object.fromEntries(found.map(([l, doc]) => [l, doc])), texts.source);
		return { doc: result.doc, conversion: { report: result.report, locales: found.map(([l]) => l) } };
	}

	/** Starts a new entity (e.g. a new chapter) in every locale. */
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
		// a new entity is unsaved: nothing stored yet
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
		// old "true" / "false" strings of bool fields → booleans (repair-booleans.ts); the repaired locales are unsaved
		for (const l of this.locales) repairBooleans(this.data[l]);
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
		this.lastEditAt = now;
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

	/** locales that have a document */
	get present(): string[] {
		return this.locales.filter((l) => this.data[l] !== null && this.data[l] !== undefined);
	}

	get(locale: string, path: Path) {
		return getAt(this.data[locale], path);
	}

	/** An edit of the (neutral) document: one undo step (calls with the same `key` within ~1s merge). */
	edit(change: (file: any) => void, key?: string) {
		const file = this.data[this.primary];
		if (!file) return;
		this.checkpoint(key);
		change(file);
	}

	/** Sets a field of the (neutral) document; `undefined` removes it. */
	set(path: Path, value: any, key?: string) {
		const file = this.data[this.primary];
		if (!file) return;
		this.checkpoint(key ?? path.join('.'));
		if (value !== undefined) {
			setAt(file, path, clone(value));
			return;
		}
		const parent = getAt(file, path.slice(0, -1));
		if (parent && typeof parent === 'object') delete parent[path[path.length - 1]];
	}

	/**
	 * One language of a text leaf: a neutral document's locale map at `path` (an older plain value counts as the
	 * source's); a per-locale document's value in that locale.
	 */
	text(path: Path, locale: string): any {
		if (!this.texts) return this.get(locale, path);
		const value = this.get(this.primary, path);
		if (value !== null && typeof value === 'object' && !Array.isArray(value)) return value[locale];
		return locale === this.texts.source ? value : undefined;
	}

	/** A group (chapter group or scene) of a neutral document as the reader gets it in `locale` (texts resolved; previews) */
	resolvedGroup(path: Path, locale: string): Group | undefined {
		const group = this.get(this.primary, path);
		if (!group || !this.texts) return group;
		return resolveGroup(group, locale, this.texts);
	}

	/** A slice of a neutral document as the reader gets it in `locale` (texts resolved; the canvas) */
	resolvedSlice(path: Path, locale: string): Slice | undefined {
		const slice = this.get(this.primary, path);
		if (!slice || !this.texts) return slice;
		return resolveSlice(slice, locale, this.texts);
	}

	/** whether a text leaf has a text in a locale (blank = none, like the reader) */
	hasText(path: Path, locale: string): boolean {
		return isLocaleValue(this.text(path, locale));
	}

	/**
	 * Writes one language of a text leaf: a neutral document's locale map (a blank text leaves that language out – it
	 * falls back like a missing one); a per-locale document's value in that locale.
	 */
	setText(path: Path, locale: string, value: string | string[], key?: string) {
		if (!this.texts) return this.setLocal(locale, path, value, key);
		const file = this.data[this.primary];
		if (!file) return;
		this.checkpoint(key ?? `text:${locale}:${path.join('.')}`);
		const current = getAt(file, path);
		const map: Record<string, unknown> =
			current !== null && typeof current === 'object' && !Array.isArray(current)
				? current
				: current === undefined || current === null || current === ''
					? {}
					: { [this.texts.source]: current };
		if (isLocaleValue(value) || (typeof value === 'string' && value !== '')) map[locale] = clone(value);
		else delete map[locale];
		setAt(file, path, map);
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

	/** Copies the source locale's document to a locale that has none. */
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
					const result = await putDraft(this.key(l), content, force ? undefined : this.base[l], message);
					this.base[l] = result.version.id;
					this.changedAt[l] = result.version.created_at ?? new Date().toISOString();
					this.source[l] = 'store';
					this.saved[l] = JSON.stringify(content);
					done.push(l);
				} catch (e) {
					if (e instanceof ApiError && e.status === 409) throw new SaveConflict(l, e.body?.latest ?? null);
					throw new SaveError(l, e);
				}
			}
			this.lastSavedAt = Date.now();
			if (!this.dirty) this.clearAutosave();
			return done;
		} catch (e) {
			// a conflict is the session's bar (Reload / Overwrite), not a load error: `error` replaces the whole editor
			if (!(e instanceof SaveConflict)) this.error = e instanceof Error ? e.message : String(e);
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
				const result = await publishVersion(this.key(l), this.base[l] ?? undefined);
				this.published[l] = result.version.id;
				done.push(l);
			}
			return done;
		} finally {
			this.saving = false;
		}
	}

	// ── Local autosave (browser only, per entity) ─────────────────────────────────────────────────────────

	get autosaveKey() {
		return `${AUTOSAVE_PREFIX}${this.entity}`;
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
