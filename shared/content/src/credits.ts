// Credits: who made a toon, each person with their role(s). Language-neutral files with translated labels inside:
//
//   scutoons/_/people.json   PeopleRegistry  – every person, global (the same people work on several toons)
//   scutoons/_/roles.json    RoleVocabulary  – the shared role list (id, category, label per locale)
//   <toon>/_/credits.json    ToonCredits     – ordered sections of one toon (+ its custom roles)
//
// `scutoons` is a pseudo-toon of the content store (CREDITS_SCOPE) for platform-wide files; `_` is the store's
// language-neutral locale (NEUTRAL_LOCALE). The viewer's credits page and the creator's preview render the same
// `resolveCredits()` output with the `CreditsView` component of @scutoons/reader.
import { Language } from './enums.ts';
import type { Path } from './paths.ts';
import type { Level, Problem } from './validate.ts';

/** Text per locale: `{ de: "Kostüm", en: "Costume Design" }`. */
export type LocaleText = Record<string, string>;

export interface PersonLink {
	label: string;
	url: string;
}

export interface CreditPerson {
	/** stable id, referenced by the credits of every toon (`^[a-z0-9][a-z0-9-]*$`) */
	id: string;
	/** name as credited */
	name: string;
	/** sort key when lists are sorted by name, e.g. "Mozart, Mandy" */
	sort_name?: string;
	/** other spellings / artist names (found by the search, used by the migration) */
	aliases?: string[];
	links?: PersonLink[];
	/** CDN path of a portrait, e.g. `people/mandy-mozart.jpg` */
	image?: string;
	bio?: LocaleText;
	/** set when this person was merged into another one: every reference resolves to that id */
	merged_into?: string;
}

export interface PeopleRegistry {
	people: CreditPerson[];
}

export enum RoleCategory {
	STORY = 'story',
	ART = 'art',
	MUSIC = 'music',
	PERFORMANCE = 'performance',
	PRODUCTION = 'production',
	TRANSLATION = 'translation',
	OTHER = 'other'
}

/** Display order of the categories (toon credits are grouped in this order). */
export const ROLE_CATEGORIES: RoleCategory[] = [
	RoleCategory.STORY,
	RoleCategory.ART,
	RoleCategory.MUSIC,
	RoleCategory.PERFORMANCE,
	RoleCategory.PRODUCTION,
	RoleCategory.TRANSLATION,
	RoleCategory.OTHER
];

export interface CreditRole {
	/** `^[a-z0-9][a-z0-9-]*$`; custom roles of a toon must not reuse a global id */
	id: string;
	category: RoleCategory;
	label: LocaleText;
}

export interface RoleVocabulary {
	roles: CreditRole[];
}

/** One role of a person in a section. */
export interface CreditRoleEntry {
	/** id from the vocabulary or the toon's custom roles */
	role: string;
	/** specifics, e.g. `{ de: "Trompete", en: "Trumpets" }` → "Musician (Trumpets)" */
	detail?: LocaleText;
	/** for translation roles: the target language → "Translation English" */
	language?: Language;
}

export interface CreditEntry {
	person: string;
	roles: CreditRoleEntry[];
}

export type CreditSectionKind = 'toon' | 'season' | 'chapter' | 'song' | 'performance' | 'thanks';
export const CREDIT_SECTION_KINDS: CreditSectionKind[] = ['toon', 'season', 'chapter', 'song', 'performance', 'thanks'];

interface SectionBase {
	/** stable id within the file (list keys, drag and drop) */
	id: string;
	/** ordered people with their roles */
	entries: CreditEntry[];
}

/** The toon's general credits: `lead` = "Created by" (shown first, prominently), `entries` grouped by role category. */
export interface ToonCreditSection extends SectionBase {
	kind: 'toon';
	lead: CreditEntry[];
}
export interface SeasonCreditSection extends SectionBase {
	kind: 'season';
	number: number;
}
export interface ChapterCreditSection extends SectionBase {
	kind: 'chapter';
	number: number;
}
export interface SongCreditSection extends SectionBase {
	kind: 'song';
	title: LocaleText;
}
/** A live performance / recording cast. */
export interface PerformanceCreditSection extends SectionBase {
	kind: 'performance';
	title: LocaleText;
	venue?: LocaleText;
	/** free text as printed, e.g. "11/2021" */
	date?: string;
}
/** Special thanks: people without roles (roles are optional here). */
export interface ThanksCreditSection extends SectionBase {
	kind: 'thanks';
}

export type CreditSection =
	| ToonCreditSection
	| SeasonCreditSection
	| ChapterCreditSection
	| SongCreditSection
	| PerformanceCreditSection
	| ThanksCreditSection;

/** `<toon>/_/credits.json` */
export interface ToonCredits {
	toon: string;
	/** roles only this toon uses (ids must not clash with the vocabulary) */
	roles?: CreditRole[];
	sections: CreditSection[];
}

// ── Store paths ───────────────────────────────────────────────────────────────────────────────────────────

/** Pseudo-toon of the content store for platform-wide files (people, roles). */
export const CREDITS_SCOPE = 'scutoons';
export const PEOPLE_FILE = 'people.json';
export const ROLES_FILE = 'roles.json';
export const CREDITS_FILE = 'credits.json';

export const CREDIT_ID = /^[a-z0-9][a-z0-9-]*$/;

/** "Chris Nova Ludwig" → "chris-nova-ludwig" (ASCII, umlauts transliterated). */
export function slugId(text: string): string {
	const slug = text
		.toLowerCase()
		.replace(/ä/g, 'ae')
		.replace(/ö/g, 'oe')
		.replace(/ü/g, 'ue')
		.replace(/ß/g, 'ss')
		.normalize('NFKD')
		.replace(/[̀-ͯ]/g, '')
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '');
	return slug || 'x';
}

/** `base`, or `base-2`, `base-3` … when taken. */
export function uniqueId(base: string, taken: Iterable<string>): string {
	const set = new Set(taken);
	if (!set.has(base)) return base;
	for (let i = 2; ; i++) if (!set.has(`${base}-${i}`)) return `${base}-${i}`;
}

export const emptyCredits = (toon: string): ToonCredits => ({
	toon,
	roles: [],
	sections: [{ id: 'toon', kind: 'toon', lead: [], entries: [] }]
});

// ── Labels of the rendered page (content vocabulary like the roles; apps may override) ────────────────────────

export interface CreditsLabels {
	title: string;
	createdBy: string;
	crew: string;
	categories: Record<RoleCategory, string>;
	/** `{number}` is replaced */
	season: string;
	/** `{number}` is replaced; `{title}` = the chapter title when known */
	chapter: string;
	songs: string;
	performances: string;
	venue: string;
	date: string;
	thanks: string;
	/** the season's supporters (logos) at the end of the fine print */
	supportedBy: string;
}

export const CREDITS_LABELS: Record<string, CreditsLabels> = {
	de: {
		title: 'Credits',
		createdBy: 'Erstellt von',
		crew: 'Crew',
		categories: {
			story: 'Geschichte & Text',
			art: 'Bild & Gestaltung',
			music: 'Musik & Sound',
			performance: 'Stimmen & Darbietung',
			production: 'Produktion',
			translation: 'Übersetzung',
			other: 'Weitere'
		},
		season: 'Staffel {number}',
		chapter: 'Kapitel {number}',
		songs: 'Songs',
		performances: 'Besetzungen',
		venue: 'Ort',
		date: 'Datum',
		thanks: 'Besonderer Dank',
		supportedBy: 'Unterstützt von'
	},
	en: {
		title: 'Credits',
		createdBy: 'Created by',
		crew: 'Crew',
		categories: {
			story: 'Story & text',
			art: 'Art & visuals',
			music: 'Music & sound',
			performance: 'Voices & performance',
			production: 'Production',
			translation: 'Translation',
			other: 'Other'
		},
		season: 'Season {number}',
		chapter: 'Chapter {number}',
		songs: 'Songs',
		performances: 'Performance casts',
		venue: 'Venue',
		date: 'Date',
		thanks: 'Special thanks',
		supportedBy: 'Supported by'
	}
};

export const creditsLabels = (locale: string): CreditsLabels => CREDITS_LABELS[locale] ?? CREDITS_LABELS.en;

// ── Resolving (for rendering) ─────────────────────────────────────────────────────────────────────────────────

/** Text of a locale, else the first fallback that has one, else any. */
export function pickText(text: LocaleText | undefined, locale: string, fallbacks: string[] = []): string {
	if (!text) return '';
	for (const l of [locale, ...fallbacks]) if (text[l]?.trim()) return text[l];
	return Object.values(text).find((v) => v?.trim()) ?? '';
}

const INTL_CODES: Record<string, string> = { jp: 'ja', kr: 'ko' };

/** "en" in German → "Englisch" (Intl), the code itself when Intl can't name it. */
export function languageName(language: string, locale: string): string {
	try {
		const names = new Intl.DisplayNames([INTL_CODES[locale] ?? locale], { type: 'language' });
		const name = names.of(INTL_CODES[language] ?? language);
		return name ? name.charAt(0).toLocaleUpperCase(locale) + name.slice(1) : language;
	} catch {
		return language;
	}
}

/** All roles by id: the vocabulary, then the toon's custom roles (custom roles win on a clash). */
export function roleIndex(vocabulary: RoleVocabulary | null | undefined, credits?: ToonCredits | null): Map<string, CreditRole & { custom?: boolean }> {
	const index = new Map<string, CreditRole & { custom?: boolean }>();
	for (const role of vocabulary?.roles ?? []) if (role?.id) index.set(role.id, role);
	for (const role of credits?.roles ?? []) if (role?.id) index.set(role.id, { ...role, custom: true });
	return index;
}

export function peopleIndex(registry: PeopleRegistry | null | undefined): Map<string, CreditPerson> {
	const index = new Map<string, CreditPerson>();
	for (const person of registry?.people ?? []) if (person?.id) index.set(person.id, person);
	return index;
}

/** The person an id stands for, following `merged_into` (null when unknown). */
export function resolvePerson(people: Map<string, CreditPerson>, id: string): CreditPerson | null {
	let person = people.get(id) ?? null;
	const seen = new Set<string>();
	while (person?.merged_into && !seen.has(person.id)) {
		seen.add(person.id);
		person = people.get(person.merged_into) ?? null;
	}
	return person;
}

/** "Übersetzung Englisch", "Musiker:in (Trompete)" – one role of an entry as shown to readers. */
export function roleText(entry: CreditRoleEntry, role: CreditRole | undefined, locale: string, fallbacks: string[] = []): string {
	let text = role ? pickText(role.label, locale, fallbacks) || role.id : entry.role;
	if (entry.language) text += ` ${languageName(entry.language, locale)}`;
	const detail = pickText(entry.detail, locale, fallbacks);
	if (detail) text += ` (${detail})`;
	return text;
}

export interface ResolvedPerson {
	id: string;
	name: string;
	links: PersonLink[];
	image?: string;
	bio?: string;
}
export interface ResolvedEntry {
	person: ResolvedPerson;
	roles: string[];
}
export interface ResolvedGroup {
	category: RoleCategory;
	label: string;
	entries: ResolvedEntry[];
}
export interface ResolvedSection {
	id: string;
	kind: CreditSectionKind;
	heading: string;
	/** heading of a run of sections of the same kind (songs, performances); set on the first of the run */
	runHeading?: string;
	meta: { label: string; value: string }[];
	/** toon sections: the entries grouped by role category */
	groups: ResolvedGroup[];
	entries: ResolvedEntry[];
}
export interface ResolvedCredits {
	labels: CreditsLabels;
	lead: ResolvedEntry[];
	sections: ResolvedSection[];
}

export interface ResolveOptions {
	/** locales tried when a text is missing in `locale` (e.g. the toon's source language) */
	fallbacks?: string[];
	/** chapter number → title (for chapter section headings) */
	chapterTitles?: Record<number, string>;
	labels?: CreditsLabels;
}

/** The credits of a toon in one locale, ready to render (unknown people are skipped, unknown roles shown as id). */
export function resolveCredits(
	credits: ToonCredits,
	registry: PeopleRegistry | null | undefined,
	vocabulary: RoleVocabulary | null | undefined,
	locale: string,
	options: ResolveOptions = {}
): ResolvedCredits {
	const fallbacks = options.fallbacks ?? ['en', 'de'];
	const labels = options.labels ?? creditsLabels(locale);
	const people = peopleIndex(registry);
	const roles = roleIndex(vocabulary, credits);
	const fill = (template: string, values: Record<string, string | number>) =>
		template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ''));

	const person = (id: string): ResolvedPerson | null => {
		const p = resolvePerson(people, id);
		if (!p) return null;
		const bio = pickText(p.bio, locale, fallbacks);
		return {
			id: p.id,
			name: p.name,
			links: (p.links ?? []).filter((l) => l?.url),
			...(p.image ? { image: p.image } : {}),
			...(bio ? { bio } : {})
		};
	};
	const entries = (list: CreditEntry[] | undefined, filter?: (role: CreditRole | undefined) => boolean): ResolvedEntry[] => {
		const out: ResolvedEntry[] = [];
		for (const entry of list ?? []) {
			const p = person(entry?.person);
			if (!p) continue;
			const texts = (entry.roles ?? [])
				.filter((r) => !filter || filter(roles.get(r.role)))
				.map((r) => roleText(r, roles.get(r.role), locale, fallbacks));
			if (filter && !texts.length) continue;
			out.push({ person: p, roles: texts });
		}
		return out;
	};

	const lead: ResolvedEntry[] = [];
	const sections: ResolvedSection[] = [];
	let previousKind: CreditSectionKind | null = null;
	for (const section of credits?.sections ?? []) {
		if (!section) continue;
		const base = { id: section.id, kind: section.kind, heading: '', meta: [] as { label: string; value: string }[], groups: [] as ResolvedGroup[], entries: [] as ResolvedEntry[] };
		switch (section.kind) {
			case 'toon': {
				lead.push(...entries(section.lead));
				base.heading = labels.crew;
				base.groups = ROLE_CATEGORIES.map((category) => ({
					category,
					label: labels.categories[category],
					entries: entries(section.entries, (role) => (role?.category ?? RoleCategory.OTHER) === category)
				})).filter((g) => g.entries.length);
				break;
			}
			case 'season':
				base.heading = fill(labels.season, { number: section.number });
				base.entries = entries(section.entries);
				break;
			case 'chapter': {
				const title = options.chapterTitles?.[section.number];
				base.heading = fill(labels.chapter, { number: section.number }) + (title ? `: ${title}` : '');
				base.entries = entries(section.entries);
				break;
			}
			case 'song':
				base.heading = pickText(section.title, locale, fallbacks);
				base.entries = entries(section.entries);
				break;
			case 'performance': {
				base.heading = pickText(section.title, locale, fallbacks);
				const venue = pickText(section.venue, locale, fallbacks);
				if (venue) base.meta.push({ label: labels.venue, value: venue });
				if (section.date) base.meta.push({ label: labels.date, value: section.date });
				base.entries = entries(section.entries);
				break;
			}
			case 'thanks':
				base.heading = labels.thanks;
				base.entries = entries(section.entries);
				break;
			default:
				continue;
		}
		const run = section.kind === 'song' ? labels.songs : section.kind === 'performance' ? labels.performances : undefined;
		const resolved: ResolvedSection = { ...base, ...(run && previousKind !== section.kind ? { runHeading: run } : {}) };
		previousKind = section.kind;
		if (section.kind === 'toon' && !resolved.groups.length) continue;
		if (section.kind !== 'toon' && !resolved.entries.length) continue;
		sections.push(resolved);
	}
	return { labels, lead, sections };
}

// ── Editing helpers ───────────────────────────────────────────────────────────────────────────────────────────

/** Every entry list of a credits file with its path (toon sections: `lead` and `entries`). */
export function forEachEntryList(credits: ToonCredits | null | undefined, fn: (list: CreditEntry[], path: Path, section: CreditSection) => void) {
	(credits?.sections ?? []).forEach((section, i) => {
		if (!section) return;
		if (section.kind === 'toon') fn(section.lead ?? [], ['sections', i, 'lead'], section);
		fn(section.entries ?? [], ['sections', i, 'entries'], section);
	});
}

/**
 * Replaces person `from` by `to` in a credits file (merging duplicates). An entry of `from` in a list that already has
 * `to` is folded into it (roles appended, duplicates dropped). Returns the number of references changed.
 */
export function replacePerson(credits: ToonCredits, from: string, to: string): number {
	let changed = 0;
	forEachEntryList(credits, (list) => {
		for (let i = 0; i < list.length; i++) {
			if (list[i].person !== from) continue;
			changed++;
			const target = list.find((e) => e.person === to);
			if (target) {
				for (const role of list[i].roles ?? []) {
					if (!target.roles.some((r) => JSON.stringify(r) === JSON.stringify(role))) target.roles.push(role);
				}
				list.splice(i, 1);
				i--;
			} else {
				list[i].person = to;
			}
		}
	});
	return changed;
}

/** How often each person id is referenced in a credits file. */
export function personUsage(credits: ToonCredits | null | undefined): Map<string, number> {
	const usage = new Map<string, number>();
	forEachEntryList(credits, (list) => list.forEach((e) => usage.set(e.person, (usage.get(e.person) ?? 0) + 1)));
	return usage;
}

// ── Validation ────────────────────────────────────────────────────────────────────────────────────────────────

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const isStr = (v: unknown): v is string => typeof v === 'string';
const LANGUAGES = Object.values(Language) as string[];
const URL_PATTERN = /^(https?:\/\/|mailto:)\S+$/i;

function collector(file: string) {
	const problems: Problem[] = [];
	const add = (level: Level, path: Path, message: string) => problems.push({ level, file, locale: null, path, message });
	return { problems, add };
}

function checkLocaleText(add: (l: Level, p: Path, m: string) => void, value: unknown, path: Path, what: string, languages: string[], required: boolean) {
	if (value === undefined) {
		if (required) add('error', path, `${what}: text is missing`);
		return;
	}
	if (!isObj(value)) return add('error', path, `${what} must be { <locale>: text }`);
	for (const [l, text] of Object.entries(value)) if (!isStr(text)) add('error', [...path, l], `${what}: ${l} must be a text`);
	const present = languages.filter((l) => isStr(value[l]) && value[l].trim());
	if (required && !present.length && !Object.values(value).some((t) => isStr(t) && t.trim())) add('error', path, `${what}: text is empty`);
	for (const l of languages) if (!(isStr(value[l]) && value[l].trim()) && Object.values(value).some((t) => isStr(t) && t.trim())) add('warning', [...path, l], `${what}: missing ${l} translation`);
}

/** people.json */
export function validatePeople(registry: unknown, languages: string[] = [], file = PEOPLE_FILE): Problem[] {
	const { problems, add } = collector(file);
	if (!isObj(registry) || !Array.isArray(registry.people)) {
		add('error', [], 'people file must be { people: [...] }');
		return problems;
	}
	const ids = new Set<string>(registry.people.map((p: any) => p?.id).filter(isStr));
	const seen = new Set<string>();
	const names = new Map<string, string>();
	registry.people.forEach((p: any, i: number) => {
		const path: Path = ['people', i];
		if (!isObj(p)) return add('error', path, 'person must be an object');
		const who = isStr(p.name) && p.name ? p.name : `person ${i + 1}`;
		if (!isStr(p.id) || !CREDIT_ID.test(p.id)) add('error', [...path, 'id'], `${who}: id must match ${CREDIT_ID.source}`);
		else if (seen.has(p.id)) add('error', [...path, 'id'], `${who}: id "${p.id}" is used twice`);
		else seen.add(p.id);
		if (!isStr(p.name) || !p.name.trim()) add('error', [...path, 'name'], `person ${p.id ?? i + 1}: name is empty`);
		else if (!p.merged_into) {
			const key = p.name.trim().toLowerCase();
			if (names.has(key)) add('warning', [...path, 'name'], `"${p.name}" is in the registry twice (${names.get(key)}, ${p.id}) – merge them`);
			else names.set(key, p.id);
		}
		if (p.sort_name !== undefined && !isStr(p.sort_name)) add('error', [...path, 'sort_name'], `${who}: sort_name must be a text`);
		if (p.aliases !== undefined && !(Array.isArray(p.aliases) && p.aliases.every(isStr))) add('error', [...path, 'aliases'], `${who}: aliases must be a list of names`);
		if (p.image !== undefined && (!isStr(p.image) || !/\.(jpe?g|png|webp|gif|svg)$/i.test(p.image))) add('warning', [...path, 'image'], `${who}: image must be a CDN image path`);
		if (p.links !== undefined) {
			if (!Array.isArray(p.links)) add('error', [...path, 'links'], `${who}: links must be a list`);
			else
				p.links.forEach((l: any, j: number) => {
					if (!isObj(l) || !isStr(l.url) || !URL_PATTERN.test(l.url)) add('error', [...path, 'links', j], `${who}: link ${j + 1} needs an http(s) url`);
					else if (!isStr(l.label) || !l.label.trim()) add('warning', [...path, 'links', j, 'label'], `${who}: link ${j + 1} has no label`);
				});
		}
		if (p.bio !== undefined) checkLocaleText(add, p.bio, [...path, 'bio'], `${who}: bio`, languages, false);
		if (p.merged_into !== undefined && (!isStr(p.merged_into) || !ids.has(p.merged_into) || p.merged_into === p.id)) {
			add('error', [...path, 'merged_into'], `${who}: merged into an unknown person "${p.merged_into}"`);
		}
	});
	return problems;
}

function checkRoles(add: (l: Level, p: Path, m: string) => void, roles: unknown, path: Path, languages: string[], taken?: Set<string>) {
	if (!Array.isArray(roles)) return add('error', path, 'roles must be a list');
	const seen = new Set<string>();
	roles.forEach((r: any, i: number) => {
		const rp: Path = [...path, i];
		if (!isObj(r)) return add('error', rp, 'role must be an object');
		const who = `role ${isStr(r.id) ? r.id : i + 1}`;
		if (!isStr(r.id) || !CREDIT_ID.test(r.id)) add('error', [...rp, 'id'], `${who}: id must match ${CREDIT_ID.source}`);
		else if (seen.has(r.id)) add('error', [...rp, 'id'], `${who}: id is used twice`);
		else if (taken?.has(r.id)) add('error', [...rp, 'id'], `${who}: custom role reuses a global role id`);
		else seen.add(r.id);
		if (!ROLE_CATEGORIES.includes(r.category)) add('error', [...rp, 'category'], `${who}: category must be one of ${ROLE_CATEGORIES.join(', ')}`);
		checkLocaleText(add, r.label, [...rp, 'label'], `${who}: label`, languages, true);
	});
}

/** roles.json (`languages`: every language any toon uses, missing labels are warnings) */
export function validateRoleVocabulary(vocabulary: unknown, languages: string[] = [], file = ROLES_FILE): Problem[] {
	const { problems, add } = collector(file);
	if (!isObj(vocabulary) || !Array.isArray(vocabulary.roles)) {
		add('error', [], 'roles file must be { roles: [...] }');
		return problems;
	}
	checkRoles(add, vocabulary.roles, ['roles'], languages);
	return problems;
}

export interface CreditsContext {
	people?: PeopleRegistry | null;
	vocabulary?: RoleVocabulary | null;
	/** the toon's languages: role labels / titles must exist in each */
	languages: string[];
	/** chapter / season numbers that exist (unknown numbers are warnings) */
	chapters?: number[];
	seasons?: number[];
	file?: string;
}

/** <toon>/_/credits.json against the registry and the vocabulary */
export function validateCredits(credits: unknown, ctx: CreditsContext): Problem[] {
	const { problems, add } = collector(ctx.file ?? CREDITS_FILE);
	if (!isObj(credits) || !Array.isArray(credits.sections)) {
		add('error', [], 'credits file must be { toon, roles?, sections: [...] }');
		return problems;
	}
	const people = peopleIndex(ctx.people);
	const globalIds = new Set((ctx.vocabulary?.roles ?? []).map((r) => r.id));
	if (credits.roles !== undefined) checkRoles(add, credits.roles, ['roles'], ctx.languages, globalIds);
	const roles = roleIndex(ctx.vocabulary, credits as ToonCredits);
	const usedRoles = new Map<string, Path>();

	const sectionIds = new Set<string>();
	const sectionKeys = new Map<string, number>();
	if (!credits.sections.some((s: any) => s?.kind === 'toon')) add('warning', ['sections'], 'no toon section (general credits, "Created by")');
	credits.sections.forEach((s: any, i: number) => {
		const sp: Path = ['sections', i];
		if (!isObj(s)) return add('error', sp, 'section must be an object');
		if (!CREDIT_SECTION_KINDS.includes(s.kind)) return add('error', [...sp, 'kind'], `section ${i + 1}: kind must be one of ${CREDIT_SECTION_KINDS.join(', ')}`);
		const name = sectionName(s, ctx.languages[0] ?? 'en');
		if (!isStr(s.id) || !s.id) add('error', [...sp, 'id'], `${name}: section id is missing`);
		else if (sectionIds.has(s.id)) add('error', [...sp, 'id'], `${name}: section id "${s.id}" is used twice`);
		else sectionIds.add(s.id);

		if (s.kind === 'season' || s.kind === 'chapter') {
			if (!Number.isInteger(s.number) || s.number < 1) add('error', [...sp, 'number'], `${name}: number must be a positive integer`);
			else {
				const known = s.kind === 'season' ? ctx.seasons : ctx.chapters;
				if (known && !known.includes(s.number)) add('warning', [...sp, 'number'], `${name}: the toon has no ${s.kind} ${s.number}`);
			}
		}
		if (s.kind === 'song' || s.kind === 'performance') checkLocaleText(add, s.title, [...sp, 'title'], `${name}: title`, ctx.languages, true);
		if (s.kind === 'performance') {
			if (s.venue !== undefined) checkLocaleText(add, s.venue, [...sp, 'venue'], `${name}: venue`, ctx.languages, false);
			if (s.date !== undefined && !isStr(s.date)) add('error', [...sp, 'date'], `${name}: date must be a text`);
		}
		const key = s.kind === 'toon' || s.kind === 'thanks' ? s.kind : s.kind === 'season' || s.kind === 'chapter' ? `${s.kind}:${s.number}` : `${s.kind}:${JSON.stringify(s.title)}`;
		if (sectionKeys.has(key)) add('warning', sp, `${name}: same section as #${(sectionKeys.get(key) ?? 0) + 1} – merge them`);
		else sectionKeys.set(key, i);

		const lists: [string, unknown][] = s.kind === 'toon' ? [['lead', s.lead], ['entries', s.entries]] : [['entries', s.entries]];
		let total = 0;
		for (const [listName, list] of lists) {
			const lp: Path = [...sp, listName];
			if (list === undefined && listName === 'lead') continue;
			if (!Array.isArray(list)) {
				add('error', lp, `${name}: ${listName} must be a list`);
				continue;
			}
			total += list.length;
			const persons = new Set<string>();
			list.forEach((e: any, j: number) => {
				const ep: Path = [...lp, j];
				if (!isObj(e)) return add('error', ep, `${name}: entry ${j + 1} must be { person, roles }`);
				const p = isStr(e.person) ? people.get(e.person) : undefined;
				const who = p?.name ?? String(e.person);
				if (!isStr(e.person) || !p) add('error', [...ep, 'person'], `${name}: unknown person "${e.person}"`);
				else if (p.merged_into) add('warning', [...ep, 'person'], `${name}: ${who} was merged into "${p.merged_into}" – replace the reference`);
				if (isStr(e.person)) {
					if (persons.has(e.person)) add('warning', [...ep, 'person'], `${name}: ${who} is listed twice`);
					persons.add(e.person);
				}
				if (!Array.isArray(e.roles)) return add('error', [...ep, 'roles'], `${name}: ${who}: roles must be a list`);
				if (!e.roles.length && s.kind !== 'thanks') add('warning', [...ep, 'roles'], `${name}: ${who} has no role`);
				const roleKeys = new Set<string>();
				e.roles.forEach((r: any, k: number) => {
					const rp: Path = [...ep, 'roles', k];
					if (!isObj(r) || !isStr(r.role)) return add('error', rp, `${name}: ${who}: role ${k + 1} must be { role, detail?, language? }`);
					const role = roles.get(r.role);
					if (!role) add('error', [...rp, 'role'], `${name}: ${who}: unknown role "${r.role}"`);
					else if (!usedRoles.has(r.role)) usedRoles.set(r.role, [...rp, 'role']);
					const roleKey = JSON.stringify([r.role, r.detail ?? null, r.language ?? null]);
					if (roleKeys.has(roleKey)) add('warning', rp, `${name}: ${who}: role "${r.role}" twice`);
					roleKeys.add(roleKey);
					if (r.detail !== undefined) checkLocaleText(add, r.detail, [...rp, 'detail'], `${name}: ${who}: detail`, ctx.languages, false);
					if (r.language !== undefined && !LANGUAGES.includes(r.language)) add('error', [...rp, 'language'], `${name}: ${who}: language must be one of ${LANGUAGES.join(', ')}`);
					if (role?.category === RoleCategory.TRANSLATION && r.language === undefined) add('info', [...rp, 'language'], `${name}: ${who}: translation without a language`);
				});
			});
		}
		if (!total) add('warning', sp, `${name}: section is empty`);
	});

	// labels of every used role in every toon language
	for (const [id, path] of usedRoles) {
		const role = roles.get(id);
		if (!role) continue;
		for (const l of ctx.languages) {
			if (!(isStr(role.label?.[l]) && role.label[l].trim())) add('warning', path, `role "${id}"${role.custom ? ' (custom)' : ''}: missing ${l} label`);
		}
	}
	return problems;
}

/** "Season 1", "Song: Cloud 9", … (problem messages, editor headings) */
export function sectionName(section: Partial<CreditSection> & { kind?: string }, locale = 'en'): string {
	const s = section as any;
	switch (section?.kind) {
		case 'toon':
			return 'Toon credits';
		case 'season':
			return `Season ${s.number ?? '?'}`;
		case 'chapter':
			return `Chapter ${s.number ?? '?'}`;
		case 'song':
			return `Song "${pickText(s.title, locale) || '?'}"`;
		case 'performance':
			return `Performance "${pickText(s.title, locale) || '?'}"`;
		case 'thanks':
			return 'Special thanks';
		default:
			return 'Section';
	}
}
