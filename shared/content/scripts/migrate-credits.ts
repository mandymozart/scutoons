// Migrates the old per-locale credits of a toon into the credits model of credits.ts:
//   <api>/<toon>/{de,en}/credits.json  (general, songs, performances, specialThanks – free-text roles per language)
//   <api>/<toon>/{de,en}/season<N>.json `credits` { artist[], writer, music[], sound[] } + toon.json `authors`
// → <out>/scutoons/_/people.json, <out>/scutoons/_/roles.json, <out>/<toon>/_/credits.json (+ report on stdout).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-credits.ts \
//        [--api=viewer/static/api] [--toon=catpope] [--out=shared/content/seed] [--json]
//
// Then import (published version 1, idempotent):  php server/bin/console import-content shared/content/seed
// Production SQL:                                 php server/bin/console import-content shared/content/seed --sql
//
// Rules: people are matched by name (exact, then first + last word, then the aligned de/en slot of the same field);
// roles are aligned de ↔ en by position per person and mapped by the de/en pair to the seed vocabulary below.
// Pairs the seed doesn't know become custom roles of the toon with both labels (reported), so nothing is lost.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	CREDITS_FILE,
	CREDITS_SCOPE,
	Language,
	PEOPLE_FILE,
	ROLES_FILE,
	RoleCategory,
	slugId,
	uniqueId,
	validateCredits,
	validatePeople,
	validateRoleVocabulary,
	type CreditEntry,
	type CreditPerson,
	type CreditRole,
	type CreditRoleEntry,
	type CreditSection,
	type ToonCredits
} from '../src/index.ts';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const arg = (name: string, fallback: string) => process.argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const API = resolve(repo, arg('api', 'viewer/static/api'));
const TOON = arg('toon', 'catpope');
const OUT = resolve(repo, arg('out', 'shared/content/seed'));
const LOCALES = ['de', 'en'] as const;

// ── Seed vocabulary: [id, category, de, en, extra de/en labels that map to it] ─────────────────────────────────
type Seed = [string, RoleCategory, string, string, [string, string][]?];
const S = RoleCategory;
const SEED: Seed[] = [
	['idea', S.STORY, 'Idea', 'Idea'],
	['text', S.STORY, 'Text', 'Text'],
	['writer', S.STORY, 'Autor:in', 'Writer'],
	['script-editing', S.STORY, 'Skript Bearbeitung', 'Script Editing'],
	['artist', S.ART, 'Künstler:in', 'Artist'],
	['layout', S.ART, 'Layout für Grafik Novell', 'Layout for Graphic Novel'],
	['graphic-design', S.ART, 'Grafikdesign', 'Graphic Design'],
	['ornaments', S.ART, 'Ornamente', 'Ornaments'],
	['photography', S.ART, 'Fotografie', 'Photography'],
	['costume', S.ART, 'Kostüm', 'Costume Design'],
	['makeup-fx', S.ART, 'Mask & Spezialeffekte FX Makeup', 'Mask & Special FX Makeup'],
	['video', S.ART, 'Video', 'Video'],
	['music', S.MUSIC, 'Musik', 'Music'],
	['sound', S.MUSIC, 'Sound', 'Sound'],
	['song-writing', S.MUSIC, 'Song-Writing', 'Song-Writing'],
	['composition', S.MUSIC, 'Komposition', 'Composition'],
	['arrangement', S.MUSIC, 'Arrangement', 'Arrangement'],
	['mixing', S.MUSIC, 'Mixing', 'Mixing'],
	['sound-design', S.MUSIC, 'Sounddesign', 'Sounddesign'],
	['musician', S.MUSIC, 'Musiker:in', 'Musician', [['Musiker', 'Musician']]],
	['trumpet', S.MUSIC, 'Trumpets', 'Trumpets'],
	['hunting-horn', S.MUSIC, 'Hunting Horn', 'Hunting Horn'],
	['flute', S.MUSIC, 'Flute', 'Flute'],
	['organ', S.MUSIC, 'Orgel', 'Organs'],
	['synths', S.MUSIC, 'Synths', 'Synths'],
	['modular-synth', S.MUSIC, 'Modular Synth', 'Modular Synth'],
	['drones', S.MUSIC, 'Dronen', 'Drones'],
	['voice', S.PERFORMANCE, 'Voice', 'Voice'],
	['fx-voice', S.PERFORMANCE, 'FX Voice', 'FX Voice'],
	['interpretation', S.PERFORMANCE, 'Interpretation', 'Interpretation'],
	['choir', S.PERFORMANCE, 'Chor', 'Choir'],
	['conductor', S.PERFORMANCE, 'Dirigent:in', 'Conductor'],
	['soprano', S.PERFORMANCE, 'Soprano', 'Soprano'],
	['mezzo-soprano', S.PERFORMANCE, 'Mezzo Soprano', 'Mezzo Soprano'],
	['alto', S.PERFORMANCE, 'Alto', 'Alto'],
	['tenor', S.PERFORMANCE, 'Tenor', 'Tenor'],
	['baritone', S.PERFORMANCE, 'Bariton', 'Baritone', [['Baritone', 'Baritone'], ['Bariton', 'Bariton']]],
	['bass', S.PERFORMANCE, 'Bass', 'Bass'],
	['production', S.PRODUCTION, 'Produktion', 'Production'],
	['translation', S.TRANSLATION, 'Übersetzung', 'Translation']
];
/** de/en pairs that are a vocabulary role + a language (translation credits) */
const TRANSLATIONS: Record<string, [string, Language]> = {
	'Übersetzung Englisch|Translations English': ['translation', Language.EN]
};
/**
 * Pairs whose en label is shared by two different de labels (two jobs, one English word): kept as custom roles of the
 * toon with both labels, reported as ambiguous for the owner to decide.
 */
const AMBIGUOUS: Record<string, [string, RoleCategory, string]> = {
	'Toningenieur|Recording Engineer': ['recording-engineer', S.MUSIC, 'de "Toningenieur" (sound engineer) and "Aufnahmeleitung" (recording direction) are both "Recording Engineer" in en'],
	'Aufnahmeleitung|Recording Engineer': ['recording-direction', S.MUSIC, 'de "Aufnahmeleitung" (recording direction) and "Toningenieur" (sound engineer) are both "Recording Engineer" in en']
};

const report: string[] = [];
const ambiguous: string[] = [];
const note = (line: string) => report.push(line);

const read = (locale: string, file: string) => {
	const path = join(API, TOON, locale, file);
	return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null;
};

// ── Roles ───────────────────────────────────────────────────────────────────────────────────────────────────────
const vocabulary: CreditRole[] = SEED.map(([id, category, de, en]) => ({ id, category, label: { de, en } }));
const pairToRole = new Map<string, CreditRoleEntry>();
for (const [id, , de, en, extra] of SEED) for (const [d, e] of [[de, en], ...(extra ?? [])]) pairToRole.set(`${d}|${e}`, { role: id });
for (const [pair, [role, language]] of Object.entries(TRANSLATIONS)) pairToRole.set(pair, { role, language });
const customRoles: CreditRole[] = [];
const roleUse = new Map<string, number>();
const seenPairs = new Map<string, string>();

function mapRole(de: string | undefined, en: string | undefined, where: string): CreditRoleEntry {
	if (de === undefined || en === undefined) {
		ambiguous.push(`${where}: role "${de ?? en}" exists only in ${de === undefined ? 'en' : 'de'} (no counterpart at the same position)`);
	}
	const d = de ?? en ?? '';
	const e = en ?? de ?? '';
	const pair = `${d}|${e}`;
	let entry = pairToRole.get(pair);
	if (!entry) {
		const known = AMBIGUOUS[pair];
		const id = known?.[0] ?? uniqueId(`custom-${slugId(e)}`, [...vocabulary, ...customRoles].map((r) => r.id));
		if (!customRoles.some((r) => r.id === id)) {
			customRoles.push({ id, category: known?.[1] ?? S.OTHER, label: { de: d, en: e } });
			ambiguous.push(`custom role "${id}" (de "${d}" / en "${e}")${known ? `: ${known[2]}` : ': not in the seed vocabulary'}`);
		}
		entry = { role: id };
		pairToRole.set(pair, entry);
	}
	const vocab = vocabulary.find((r) => r.id === entry.role);
	if (vocab && (vocab.label.de !== d || vocab.label.en !== e) && !entry.language && !seenPairs.has(pair)) {
		note(`normalized label: de "${d}" / en "${e}" → role "${vocab.id}" (${vocab.label.de} / ${vocab.label.en})`);
	}
	seenPairs.set(pair, entry.role);
	roleUse.set(entry.role, (roleUse.get(entry.role) ?? 0) + 1);
	return { ...entry };
}

// ── People ──────────────────────────────────────────────────────────────────────────────────────────────────────
const people: CreditPerson[] = [];
const byName = new Map<string, CreditPerson>();
const words = (name: string) => name.trim().split(/\s+/);
const firstLast = (name: string) => {
	const w = words(name.toLowerCase());
	return w.length > 1 ? `${w[0]} ${w[w.length - 1]}` : w[0];
};

/** exact name → person; `loose`: also first + last word ("Thomas Dalby" = "Thomas B. Dalby") */
function findPerson(name: string, loose = false): CreditPerson | null {
	const exact = byName.get(name.trim().toLowerCase());
	if (exact || !loose) return exact ?? null;
	const key = firstLast(name);
	const hits = people.filter((p) => firstLast(p.name) === key);
	return hits.length === 1 ? hits[0] : null;
}

/**
 * Owner decisions: spelling variants of one person → the correct name (the variant becomes an alias).
 * 2026-09-29: "Julia Edhofer" is "Julia Edthofer".
 */
const NAME_ALIASES: Record<string, string> = {
	'Julia Edhofer': 'Julia Edthofer'
};

function person(name: string): CreditPerson {
	const canonical = NAME_ALIASES[name.trim()];
	if (canonical) {
		const p = person(canonical);
		addAlias(p, name.trim());
		note(`name alias (owner): "${name.trim()}" → "${p.name}"`);
		return p;
	}
	const found = findPerson(name);
	if (found) return found;
	const created: CreditPerson = { id: uniqueId(slugId(name), people.map((p) => p.id)), name: name.trim() };
	people.push(created);
	byName.set(created.name.toLowerCase(), created);
	return created;
}

function addAlias(p: CreditPerson, alias: string) {
	if (alias === p.name || p.aliases?.includes(alias)) return;
	p.aliases = [...(p.aliases ?? []), alias];
	byName.set(alias.toLowerCase(), p);
}

/** One person of the old lists: de and en entry at the same position (names must agree). */
function entryOf(de: { name: string; roles: string[] } | undefined, en: { name: string; roles: string[] } | undefined, where: string): CreditEntry {
	const name = de?.name ?? en?.name ?? '';
	if (de && en && de.name !== en.name) ambiguous.push(`${where}: de "${de.name}" ≠ en "${en.name}" at the same position (kept de)`);
	if (!de || !en) ambiguous.push(`${where}: "${name}" only in ${de ? 'de' : 'en'}`);
	const p = person(name);
	const n = Math.max(de?.roles.length ?? 0, en?.roles.length ?? 0);
	const roles: CreditRoleEntry[] = [];
	for (let i = 0; i < n; i++) roles.push(mapRole(de?.roles[i], en?.roles[i], `${where} ${name}`));
	return { person: p.id, roles };
}

function list(de: any[] | undefined, en: any[] | undefined, where: string): CreditEntry[] {
	const n = Math.max(de?.length ?? 0, en?.length ?? 0);
	const out: CreditEntry[] = [];
	for (let i = 0; i < n; i++) out.push(entryOf(de?.[i], en?.[i], where));
	return out;
}

const both = (de: string | undefined, en: string | undefined) => {
	const text: Record<string, string> = {};
	if (de) text.de = de;
	if (en) text.en = en;
	return text;
};

// ── Old credits.json ────────────────────────────────────────────────────────────────────────────────────────────
const oldDe = read('de', 'credits.json');
const oldEn = read('en', 'credits.json');
if (!oldDe || !oldEn) throw new Error(`${TOON}: needs de + en credits.json in ${API}`);

const sectionIds: string[] = [];
const sid = (base: string) => {
	const id = uniqueId(slugId(base), sectionIds);
	sectionIds.push(id);
	return id;
};

const toonSection: CreditSection = { id: sid('toon'), kind: 'toon', lead: [], entries: list(oldDe.general, oldEn.general, 'general') };
const songSections: CreditSection[] = (oldDe.songs ?? []).map((song: any, i: number) => {
	const en = oldEn.songs?.[i];
	if (en && en.title !== song.title) ambiguous.push(`song ${i + 1}: de "${song.title}" / en "${en.title}" (kept as title per locale)`);
	return { id: sid(`song-${song.title}`), kind: 'song', title: both(song.title, en?.title), entries: list(song.credits, en?.credits, `song "${song.title}"`) };
});
const performanceSections: CreditSection[] = (oldDe.performances ?? []).map((cast: any, i: number) => {
	const en = oldEn.performances?.[i] ?? {};
	if ((cast.date ?? '') !== (en.date ?? '')) ambiguous.push(`performance "${cast.name}": date de "${cast.date}" ≠ en "${en.date}" (kept de)`);
	const entries = list(cast.members, en.members, `performance "${en.name ?? cast.name}"`);
	const conductor = cast.conductor ?? en.conductor;
	if (conductor) {
		if (en.conductor && cast.conductor && en.conductor !== cast.conductor) ambiguous.push(`performance "${cast.name}": conductor de "${cast.conductor}" ≠ en "${en.conductor}" (kept de)`);
		entries.unshift({ person: person(conductor).id, roles: [mapRole('Dirigent:in', 'Conductor', 'conductor')] });
	}
	return {
		id: sid(`performance-${en.name ?? cast.name}`),
		kind: 'performance',
		title: both(cast.name, en.name),
		...(cast.venue || en.venue ? { venue: both(cast.venue, en.venue) } : {}),
		...(cast.date ? { date: cast.date } : {}),
		entries
	};
});
const thanksNames: string[] = [...new Set([...(oldDe.specialThanks ?? []), ...(oldEn.specialThanks ?? [])])];
const thanksSection: CreditSection = { id: sid('thanks'), kind: 'thanks', entries: thanksNames.map((name) => ({ person: person(name).id, roles: [] })) };

// ── Season credits (+ toon authors = the "Created by" lead) ─────────────────────────────────────────────────────
const seasonSections: CreditSection[] = [];
const seasonFiles = readdirSync(join(API, TOON, 'de')).filter((f) => /^season\d+\.json$/.test(f)).sort();
const authorsDe: string[] = read('de', 'toon.json')?.authors ?? [];
const authorsEn: string[] = read('en', 'toon.json')?.authors ?? [];
if (JSON.stringify(authorsDe) !== JSON.stringify(authorsEn)) ambiguous.push(`toon.json authors differ: de ${JSON.stringify(authorsDe)} / en ${JSON.stringify(authorsEn)}`);

/** season-credit name → person: exact, first + last word, else the same slot in the other locale */
function seasonPerson(name: string, other: string | undefined, where: string): CreditPerson {
	const exact = findPerson(name);
	if (exact) return exact;
	const loose = findPerson(name, true);
	if (loose) {
		addAlias(loose, name);
		ambiguous.push(`${where}: "${name}" matched "${loose.name}" by first + last name (added as alias)`);
		return loose;
	}
	const viaOther = other ? findPerson(other, true) : null;
	if (viaOther) {
		addAlias(viaOther, name);
		ambiguous.push(`${where}: "${name}" (de) is "${viaOther.name}" in the en file at the same position → same person, "${name}" added as alias – please confirm`);
		return viaOther;
	}
	return person(name);
}

const lead = new Map<string, CreditEntry>();
for (const author of authorsDe) {
	const p = seasonPerson(author, undefined, 'toon.json authors');
	lead.set(p.id, { person: p.id, roles: [] });
}

const FIELD_ROLE: Record<string, string> = { writer: 'writer', artist: 'artist', music: 'music', sound: 'sound' };
for (const file of seasonFiles) {
	const number = Number(file.match(/\d+/)![0]);
	const de = read('de', file)?.credits ?? {};
	const en = read('en', file)?.credits ?? {};
	const rest = new Map<string, CreditEntry>();
	for (const field of ['writer', 'artist', 'music', 'sound']) {
		const namesDe: string[] = ([] as string[]).concat(de[field] ?? []);
		const namesEn: string[] = ([] as string[]).concat(en[field] ?? []);
		const n = Math.max(namesDe.length, namesEn.length);
		for (let i = 0; i < n; i++) {
			const name = namesDe[i] ?? namesEn[i];
			const where = `${file} credits.${field}[${i}]`;
			const p = seasonPerson(name, namesEn[i] !== name ? namesEn[i] : undefined, where);
			if (namesEn[i] && namesEn[i] !== name && !findPerson(namesEn[i], true)) ambiguous.push(`${where}: de "${name}" / en "${namesEn[i]}"`);
			const target = lead.get(p.id) ?? rest.get(p.id) ?? { person: p.id, roles: [] };
			if (!lead.has(p.id)) rest.set(p.id, target);
			if (!target.roles.some((r) => r.role === FIELD_ROLE[field])) {
				target.roles.push({ role: FIELD_ROLE[field] });
				roleUse.set(FIELD_ROLE[field], (roleUse.get(FIELD_ROLE[field]) ?? 0) + 1);
			}
		}
	}
	if (rest.size) seasonSections.push({ id: sid(`season-${number}`), kind: 'season', number, entries: [...rest.values()] });
	note(`${file} credits → lead (authors) + season ${number} section (${rest.size} other people)`);
}
(toonSection as any).lead = [...lead.values()];

// one entry per person and list: duplicates (e.g. after a name alias) are combined, roles in order, no repeats
let combined = 0;
for (const section of [toonSection, ...seasonSections, ...songSections, ...performanceSections, thanksSection]) {
	for (const key of ['lead', 'entries'] as const) {
		const list: CreditEntry[] | undefined = (section as any)[key];
		if (!list) continue;
		const out: CreditEntry[] = [];
		for (const entry of list) {
			const first = out.find((e) => e.person === entry.person);
			if (!first) {
				out.push(entry);
				continue;
			}
			combined++;
			for (const role of entry.roles) if (!first.roles.some((r) => JSON.stringify(r) === JSON.stringify(role))) first.roles.push(role);
		}
		(section as any)[key] = out;
	}
}
if (combined) note(`${combined} duplicate entries combined within their section`);

// ── Output ──────────────────────────────────────────────────────────────────────────────────────────────────────
const credits: ToonCredits = {
	toon: TOON,
	roles: customRoles,
	sections: [toonSection, ...seasonSections, ...songSections, ...performanceSections, thanksSection]
};
// vocabulary: every seed role (used or not, they are the shared list)
const roles = { roles: vocabulary };
const registry = { people };

const problems = [
	...validatePeople(registry, ['de', 'en']),
	...validateRoleVocabulary(roles, ['de', 'en']),
	...validateCredits(credits, { people: registry, vocabulary: roles, languages: ['de', 'en'], seasons: seasonFiles.map((f) => Number(f.match(/\d+/)![0])) })
].filter((p) => p.level !== 'info');

const write = (toon: string, file: string, data: unknown) => {
	const path = join(OUT, toon, '_', file);
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, JSON.stringify(data, null, '\t') + '\n');
	return path;
};
const files = [write(CREDITS_SCOPE, PEOPLE_FILE, registry), write(CREDITS_SCOPE, ROLES_FILE, roles), write(TOON, CREDITS_FILE, credits)];

// near-duplicate names (typos?) – kept apart, the creator's People tab can merge them
const distance = (a: string, b: string) => {
	const d = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
	for (let j = 1; j <= b.length; j++) d[0][j] = j;
	for (let i = 1; i <= a.length; i++)
		for (let j = 1; j <= b.length; j++) d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
	return d[a.length][b.length];
};
for (let i = 0; i < people.length; i++)
	for (let j = i + 1; j < people.length; j++)
		if (distance(people[i].name.toLowerCase(), people[j].name.toLowerCase()) <= 2)
			// (aliases resolved by NAME_ALIASES are one person already)
			ambiguous.push(`possible duplicate people: "${people[i].name}" / "${people[j].name}" (kept apart – merge in the creator if they are the same)`);
for (const [pair, [role, language]] of Object.entries(TRANSLATIONS)) {
	if (seenPairs.has(pair)) note(`translation: de "${pair.split('|')[0]}" / en "${pair.split('|')[1]}" → role "${role}" + language "${language}" (renders "Übersetzung Englisch" / "Translation English")`);
}

const entryCount = credits.sections.reduce((n, s) => n + s.entries.length + (s.kind === 'toon' ? s.lead.length : 0), 0);
const roleCount = credits.sections.reduce((n, s) => n + [...s.entries, ...(s.kind === 'toon' ? s.lead : [])].reduce((m, e) => m + e.roles.length, 0), 0);
const summary = {
	toon: TOON,
	people: people.length,
	aliases: people.filter((p) => p.aliases).map((p) => `${p.name} ← ${p.aliases!.join(', ')}`),
	vocabulary: vocabulary.length,
	usedVocabularyRoles: vocabulary.filter((r) => roleUse.has(r.id)).length,
	customRoles: customRoles.map((r) => `${r.id} (${r.label.de} / ${r.label.en})`),
	sections: credits.sections.map((s) => `${s.kind}${'number' in s ? ' ' + s.number : ''}${'title' in s ? ' ' + (s.title.en ?? s.title.de) : ''} (${s.entries.length}${s.kind === 'toon' ? ` + lead ${s.lead.length}` : ''})`),
	entries: entryCount,
	roleEntries: roleCount,
	ambiguous,
	notes: report,
	problems: problems.map((p) => `${p.level} ${p.file} ${p.path.join('.')}: ${p.message}`),
	files: files.map((f) => f.replace(repo + '\\', '').replace(repo + '/', ''))
};

if (process.argv.includes('--json')) console.log(JSON.stringify(summary, null, 2));
else {
	console.log(`credits migration ${TOON}: ${summary.people} people, ${summary.vocabulary} vocabulary roles (${summary.usedVocabularyRoles} used), ${customRoles.length} custom roles, ${credits.sections.length} sections, ${entryCount} entries, ${roleCount} role entries`);
	for (const [title, lines] of [['Sections', summary.sections], ['Aliases', summary.aliases], ['Custom roles', summary.customRoles], ['Ambiguous (check)', ambiguous], ['Notes', report], ['Problems', summary.problems], ['Written', summary.files]] as const) {
		if (!lines.length) continue;
		console.log(`\n${title}:`);
		for (const line of lines) console.log(`  - ${line}`);
	}
}
