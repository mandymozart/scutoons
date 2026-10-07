// Language-neutral documents (agents/NEUTRAL.md, owner 2026-10-06): one stored document per chapter, scene and season
// (`chapter:<toon>:<n>`, `scene:<toon>:<chapter>:<name>`, `season:<toon>:<n>` – no locale segment). Structure exists
// once; every text leaf is a locale map `{ de: …, en: … }`, and so are the per-language non-texts (`release`, season
// `sequence[].status`). No `language`, no chapter `status` (computed from the season when served).
//
// The reader's shape does not change: `resolve*` turns a neutral document + the served locale into today's per-locale
// `Chapter` / `Group` / `Season`. The library (server `Domain\Content\NeutralDoc`) does the same in PHP – both are
// tested against the golden fixtures in test/fixtures/neutral/ so they produce identical answers.
//
//   which locale is served   `servedLocale`: the asked one when the document has it (chapter / season: a `title` in it;
//                            scene: any text in it; a document without any text has every locale), else the toon's
//                            source language, else its other languages, else the document's first locale
//   a text in that locale    `resolveText`: the served locale → the source → the toon's other languages → the first
//                            text there is (a text is there when it isn't blank); none → ''
import type { Chapter, Dialog, Group, Season, SequenceItem, Slice, SliceItem, ToonIndex, ToonSettings } from './content.ts';
import type { AcceptedWarnings } from './accepted.ts';
import type { SensitiveFlag } from './sensitive.ts';
import type { LocaleText } from './credits.ts';
import type { Status } from './enums.ts';
import { toonLanguages } from './languages.ts';
import type {
	VNAddChoiceEvent,
	VNAskTextEvent,
	VNEventCondition,
	VNDivertEvent,
	VNEventTrigger,
	VNHideMessageEvent,
	VNJoinSceneEvent,
	VNJumpToEvent,
	VNLabelEvent,
	VNLeaveSceneEvent,
	VNSetVariableEvent,
	VNShowChoicesEvent,
	VNShowMessageEvent,
	VNShowTextBoxEvent,
	VNFxEvent,
	VNGiveItemEvent,
	VNTakeItemEvent
} from './visual-novel.ts';

// ── Stored types ──────────────────────────────────────────────────────────────────────────────────────────────
// (`AcceptedWarnings` / `SensitiveFlag` are added back explicitly: through `Omit` the schema generator makes inherited
// optional fields required)

/** A text box's text per locale; a locale's text may be several lines / paragraphs like `Dialog.text` */
export type DialogText = Record<string, string | string[]>;

/** A per-language value that is not a text (season `sequence[].status`) */
export type LocaleStatus = Record<string, Status>;

export type DialogDoc = Omit<Dialog, 'text'> & { text: DialogText };

export interface VNShowMessageEventDoc {
	type: 'showMessage';
	params: Omit<VNShowMessageEvent['params'], 'text'> & { text: LocaleText };
}

export interface VNAddChoiceEventDoc {
	type: 'addChoice';
	params: Omit<VNAddChoiceEvent['params'], 'label'> & { label: LocaleText };
}

export interface VNAskTextEventDoc {
	type: 'askText';
	params: Omit<VNAskTextEvent['params'], 'text' | 'placeholder' | 'answer' | 'default'> & { text: LocaleText; placeholder?: LocaleText; answer?: LocaleText; default?: LocaleText };
}

/** the texts (locale maps) of an event's params, by event type */
export const EVENT_TEXT_FIELDS: Readonly<Record<string, readonly string[]>> = {
	showMessage: ['text'],
	addChoice: ['label'],
	askText: ['text', 'placeholder', 'answer', 'default']
};

/** `VNEvent` with its texts as locale maps (the same members as `VNEvent`) */
export type VNEventDoc = (
	| VNShowMessageEventDoc
	| VNShowChoicesEvent
	| VNAddChoiceEventDoc
	| VNJoinSceneEvent
	| VNLeaveSceneEvent
	| VNHideMessageEvent
	| VNShowTextBoxEvent
	| VNFxEvent
	| VNJumpToEvent
	| VNLabelEvent
	| VNDivertEvent
	| VNSetVariableEvent
	| VNGiveItemEvent
	| VNTakeItemEvent
	| VNAskTextEventDoc
) & {
	id?: string;
	if?: VNEventCondition;
};

export type VNSliceEventDoc = VNEventDoc & { trigger: VNEventTrigger; layer?: number };

export type SliceItemDoc = Omit<SliceItem, 'events'> & { events?: VNEventDoc[] };

export type SliceDoc = Omit<Slice, 'dialog' | 'events' | 'items' | 'sensitive'> &
	SensitiveFlag & {
		dialog?: DialogDoc[];
		events?: VNSliceEventDoc[];
		items?: SliceItemDoc[];
	};

/** A group: inline in a chapter, or stored on its own as a scene (`scene:<toon>:<chapter>:<name>`) */
export type GroupDoc = Omit<Group, 'slices' | 'events' | 'accepted_warnings'> &
	AcceptedWarnings & {
		slices?: SliceDoc[];
		events?: VNEventDoc[];
	};

/** The stored chapter `chapter:<toon>:<n>` */
export type ChapterDoc = Omit<Chapter, 'title' | 'description' | 'release' | 'language' | 'status' | 'groups' | 'accepted_warnings'> &
	AcceptedWarnings & {
		title: LocaleText;
		description: LocaleText;
		/** per language (a translation can be released later) */
		release: LocaleText;
		groups: GroupDoc[];
	};

export type SequenceItemDoc = Omit<SequenceItem, 'status'> & {
	/** visibility of the chapter per locale (a toon may release de before en) */
	status: LocaleStatus;
};

/** The stored season `season:<toon>:<n>` */
export type SeasonDoc = Omit<Season, 'title' | 'description' | 'release' | 'language' | 'sequence' | 'accepted_warnings'> &
	AcceptedWarnings & {
		title: LocaleText;
		description?: LocaleText;
		release?: LocaleText;
		sequence: SequenceItemDoc[];
	};

/** The kinds stored as one language-neutral document (also accepted per locale until production is converted) */
export type NeutralKind = 'chapter' | 'scene' | 'season';

/**
 * What resolves for a locale: the neutral kinds + the toon entry `toon:<toon>` (its locale maps → the per-language toon
 * settings, agents/NEUTRAL.md step 4; served in the locale of its `title` like a season)
 */
export type ResolvableKind = NeutralKind | 'toon';

// ── Resolution ────────────────────────────────────────────────────────────────────────────────────────────────

export interface NeutralLanguages {
	/** the toon's source language */
	source: string;
	/** every language of the toon, source first (`toonLanguages().languages`) */
	languages?: readonly string[];
}

const isObject = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);

/** Whether a locale map's value is a text (not blank; a list counts when one of its lines isn't blank) */
export function isLocaleValue(value: unknown): boolean {
	if (typeof value === 'string') return value.trim() !== '';
	return Array.isArray(value) && value.some((line) => typeof line === 'string' && line.trim() !== '');
}

/** The locales to try for a text: the served one, the source, the toon's other languages (no duplicates) */
export function localeChain(locale: string, languages: NeutralLanguages): string[] {
	return [...new Set([locale, languages.source, ...(languages.languages ?? [])])];
}

/**
 * A locale map's value: the first locale of the chain that has one, else the first value there is; undefined when
 * there is none. Anything that is not a locale map (an older plain string) is returned as it is.
 */
export function resolveText<T = string>(value: unknown, chain: readonly string[]): T | undefined {
	if (!isObject(value)) return value as T | undefined;
	for (const locale of chain) {
		if (Object.hasOwn(value, locale) && isLocaleValue(value[locale])) return value[locale] as T;
	}
	for (const text of Object.values(value)) if (isLocaleValue(text)) return text as T;
	return undefined;
}

function addLocales(value: unknown, out: string[]) {
	if (!isObject(value)) return;
	for (const [locale, text] of Object.entries(value)) if (isLocaleValue(text) && !out.includes(locale)) out.push(locale);
}

function eventLocales(event: unknown, out: string[]) {
	if (!isObject(event) || !isObject(event.params)) return;
	for (const field of EVENT_TEXT_FIELDS[String(event.type)] ?? []) addLocales(event.params[field], out);
}

function groupLocales(group: unknown, out: string[]) {
	if (!isObject(group)) return;
	for (const slice of Array.isArray(group.slices) ? group.slices : []) {
		if (!isObject(slice)) continue;
		for (const dialog of Array.isArray(slice.dialog) ? slice.dialog : []) if (isObject(dialog)) addLocales(dialog.text, out);
		for (const event of Array.isArray(slice.events) ? slice.events : []) eventLocales(event, out);
		for (const item of Array.isArray(slice.items) ? slice.items : []) {
			if (isObject(item)) for (const event of Array.isArray(item.events) ? item.events : []) eventLocales(event, out);
		}
	}
	for (const event of Array.isArray(group.events) ? group.events : []) eventLocales(event, out);
}

/**
 * The locales a neutral document has, in its own order: chapter / season = the locales of its `title`; scene = the
 * locales of any of its texts. [] = it has no text at all (then it serves every locale).
 */
export function neutralLocales(kind: ResolvableKind, doc: unknown): string[] {
	const out: string[] = [];
	if (!isObject(doc)) return out;
	if (kind === 'scene') groupLocales(doc, out);
	else addLocales(doc.title, out);
	return out;
}

/** The locale a neutral document is served in for an asked locale (see the header) + whether that is a fallback */
export function servedLocale(kind: ResolvableKind, doc: unknown, locale: string, languages: NeutralLanguages): { locale: string; fallback: boolean } {
	const has = neutralLocales(kind, doc);
	if (!has.length) return { locale, fallback: false };
	const served = localeChain(locale, languages).find((l) => has.includes(l)) ?? has[0];
	return { locale: served, fallback: served !== locale };
}

function textField(target: Record<string, unknown>, key: string, chain: readonly string[]) {
	if (Object.hasOwn(target, key)) target[key] = resolveText(target[key], chain) ?? '';
}

function resolveEvent(event: unknown, chain: readonly string[]): unknown {
	if (!isObject(event) || !isObject(event.params)) return event;
	const fields = (EVENT_TEXT_FIELDS[String(event.type)] ?? []).filter((f) => Object.hasOwn(event.params as object, f));
	if (!fields.length) return event;
	const params = { ...event.params };
	for (const field of fields) textField(params, field, chain);
	return { ...event, params };
}

const resolveEvents = (events: unknown, chain: readonly string[]) => (Array.isArray(events) ? events.map((e) => resolveEvent(e, chain)) : events);

function resolveSliceWith(slice: unknown, chain: readonly string[]): unknown {
	if (!isObject(slice)) return slice;
	const out: Record<string, unknown> = { ...slice };
	if (Array.isArray(slice.dialog)) {
		out.dialog = slice.dialog.map((dialog) => {
			if (!isObject(dialog)) return dialog;
			const copy = { ...dialog };
			textField(copy, 'text', chain);
			return copy;
		});
	}
	if (Object.hasOwn(slice, 'events')) out.events = resolveEvents(slice.events, chain);
	if (Array.isArray(slice.items)) {
		out.items = slice.items.map((item) => (isObject(item) && Object.hasOwn(item, 'events') ? { ...item, events: resolveEvents(item.events, chain) } : item));
	}
	return out;
}

function resolveGroupWith(group: unknown, chain: readonly string[]): unknown {
	if (!isObject(group)) return group;
	const out: Record<string, unknown> = { ...group };
	if (Array.isArray(group.slices)) out.slices = group.slices.map((s) => resolveSliceWith(s, chain));
	if (Object.hasOwn(group, 'events')) out.events = resolveEvents(group.events, chain);
	return out;
}

/** A neutral slice in a locale (its text boxes' and events' texts resolved; the creator's canvas and previews) */
export function resolveSlice(slice: SliceDoc, locale: string, languages: NeutralLanguages): Slice {
	return resolveSliceWith(slice, localeChain(locale, languages)) as Slice;
}

/**
 * A text leaf as one line for a list / preview (a locale map's text in the chain – else the first there is –, a plain
 * string, or a string[] joined by spaces); '' when there is none
 */
export function previewText(value: unknown, chain: readonly string[] = []): string {
	const text = resolveText<unknown>(value, chain);
	if (Array.isArray(text)) return text.filter((t) => typeof t === 'string').join(' ');
	return typeof text === 'string' ? text : '';
}

/** A neutral group / scene in the served locale (today's `Group`) */
export function resolveGroup(doc: GroupDoc, locale: string, languages: NeutralLanguages): Group {
	return resolveGroupWith(doc, localeChain(locale, languages)) as Group;
}

/**
 * A neutral chapter in the served locale (today's `Chapter`): texts and `release` resolved, `language` = the locale,
 * `status` = the season's status of the chapter in that locale when given (the deprecated mirror older viewer builds
 * read). Groups stored as scenes keep their `ref` (the library inlines them).
 */
export function resolveChapter(doc: ChapterDoc, locale: string, languages: NeutralLanguages, status?: Status | null): Chapter {
	const chain = localeChain(locale, languages);
	const out: Record<string, unknown> = { ...doc };
	for (const key of ['title', 'description', 'release']) textField(out, key, chain);
	out.language = locale;
	if (status) out.status = status;
	if (Array.isArray(doc.groups)) out.groups = doc.groups.map((g) => resolveGroupWith(g, chain));
	return out as unknown as Chapter;
}

/** A neutral season in the served locale (today's `Season`): texts, `release`, `sequence[].status` resolved, `language` = the locale */
export function resolveSeason(doc: SeasonDoc, locale: string, languages: NeutralLanguages): Season {
	const chain = localeChain(locale, languages);
	const out: Record<string, unknown> = { ...doc };
	for (const key of ['title', 'description', 'release']) textField(out, key, chain);
	out.language = locale;
	if (Array.isArray(doc.sequence)) {
		out.sequence = doc.sequence.map((item) => {
			if (!isObject(item) || !Object.hasOwn(item, 'status')) return item;
			const copy: Record<string, unknown> = { ...item };
			const status = resolveText(item.status, chain);
			if (status === undefined || status === null) delete copy.status;
			else copy.status = status;
			return copy;
		});
	}
	return out as unknown as Season;
}

/**
 * The toon entry's per-language settings in a locale (today's `ToonSettings`, the per-language document it replaces):
 * `id` (the entry's), `title`, `language` = the locale, `release` and `description` when the entry has them, `status`
 * = the language's `languageStatus` (missing → the source's → …; none at all → no status = published). The entry's
 * language-neutral settings are merged by the library as before.
 */
export function resolveToon(entry: ToonIndex, locale: string, languages: NeutralLanguages): ToonSettings {
	const chain = localeChain(locale, languages);
	const doc = entry as unknown as Record<string, unknown>;
	const out: Record<string, unknown> = { id: doc.id, title: resolveText(doc.title, chain) ?? '', language: locale };
	if (Object.hasOwn(doc, 'release')) out.release = resolveText(doc.release, chain) ?? '';
	const status = resolveText(doc.languageStatus, chain);
	if (status !== undefined && status !== null) out.status = status;
	if (Object.hasOwn(doc, 'description')) out.description = resolveText(doc.description, chain) ?? '';
	return out as unknown as ToonSettings;
}

/**
 * The toon's name in a locale (owner decision 4: the name is the entry's `title`): `title` resolved locale → source →
 * the toon's other languages → any; an older entry without one: its deprecated `i18n.<l>.name` the same way; else the
 * id. Every list / menu / nav reads the name through this.
 */
export function toonName(entry: Partial<Pick<ToonIndex, 'id' | 'title' | 'i18n' | 'sourceLanguage' | 'languages'>> | null | undefined, locale: string): string {
	if (!entry) return '';
	const chain = localeChain(locale, toonLanguages(entry));
	const title = resolveText<unknown>(entry.title, chain);
	if (typeof title === 'string' && title.trim() !== '') return title;
	const names = Object.fromEntries(Object.entries(entry.i18n ?? {}).map(([l, v]) => [l, isObject(v) ? v.name : undefined]));
	const legacy = resolveText<unknown>(names, chain);
	return typeof legacy === 'string' && legacy.trim() !== '' ? legacy : (entry.id ?? '');
}

/**
 * A neutral document for an asked locale, as the library serves it: the served locale (+ fallback flag) and the
 * document in it (a chapter's `status` = `chapterStatus` when given, else none; a toon entry → its `ToonSettings`)
 */
export function resolveNeutral(
	kind: ResolvableKind,
	doc: unknown,
	locale: string,
	languages: NeutralLanguages,
	chapterStatus?: Status | null
): { locale: string; fallback: boolean; doc: Chapter | Group | Season | ToonSettings } {
	const served = servedLocale(kind, doc, locale, languages);
	const resolved =
		kind === 'chapter'
			? resolveChapter(doc as ChapterDoc, served.locale, languages, chapterStatus)
			: kind === 'season'
				? resolveSeason(doc as SeasonDoc, served.locale, languages)
				: kind === 'toon'
					? resolveToon(doc as ToonIndex, served.locale, languages)
					: resolveGroup(doc as GroupDoc, served.locale, languages);
	return { ...served, doc: resolved };
}
