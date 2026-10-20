// The Kanon (agents/research/KANON.md, #80): the story texts behind the documents – one big plain text per subject and
// language, creator only (never served to readers), versioned like every entity. The word is "Kanon", never "bible"
// (owner 2026-10-20). Publish = Kanon: the published version is what generators, briefs and the MCP read.
//
//   kanon:toon.<toon>:<l>                              the toon: premise, tone, world rules
//   kanon:character.<id>:<l>                           a character: backstory, who, look, rules
//   kanon:character.<id>.<persona>:<l>                 a persona: what it is, why it differs, look, rules
//   kanon:character.<id>.<persona>.<look>:<l>          a look: look, when
//   kanon:place.<id>:<l>                               a map place: backstory, scenery, rules
//
// The subject is one key segment, its parts joined by "." (no id may hold a dot, so a key never reads two ways). The
// rules per subject role: `## <heading>` lines at column 0 open a section, every section is optional, the order is
// fixed; headings are accepted in German and English. Text before the first heading is an intro (no warning).
import type { AcceptedWarnings } from './accepted.ts';

/** One Kanon text (entity `kanon:<subject>:<l>`) */
export interface KanonText extends AcceptedWarnings {
	/** the whole text, plain, exactly as typed (LF line ends) */
	text: string;
}

/** What a Kanon text is about */
export type KanonSubject =
	| { kind: 'toon'; toon: string }
	| { kind: 'character'; character: string; persona?: string; look?: string }
	| { kind: 'place'; place: string };

/** Which rules a text follows (a character's subject is a character, a persona or a look) */
export type KanonRole = 'toon' | 'character' | 'persona' | 'look' | 'place';

const SLUG = '[a-z0-9][a-z0-9_-]{0,63}';
const PART = '[a-z0-9][a-z0-9-]{0,63}';
/** a subject segment (server EntityKey::KANON_SUBJECT_PATTERN) */
export const KANON_SUBJECT = new RegExp(`^(?:toon\\.[a-z0-9_-]{1,64}|place\\.${SLUG}|character\\.${SLUG}(?:\\.${PART}){0,2})$`);

/** The subject of a subject segment (`character.catpope.human` → { character, persona }), null when it is none */
export function parseKanonSubject(segment: string): KanonSubject | null {
	if (!KANON_SUBJECT.test(segment)) return null;
	const [kind, ...rest] = segment.split('.');
	if (kind === 'toon') return rest[0] === 'scutoons' ? null : { kind, toon: rest[0] };
	if (kind === 'place') return { kind, place: rest[0] };
	const [character, persona, look] = rest;
	return { kind: 'character', character, ...(persona ? { persona } : {}), ...(look ? { look } : {}) };
}

/** The subject segment of a subject */
export function kanonSubjectSegment(s: KanonSubject): string {
	if (s.kind === 'toon') return `toon.${s.toon}`;
	if (s.kind === 'place') return `place.${s.place}`;
	return ['character', s.character, s.persona, s.persona ? s.look : undefined].filter(Boolean).join('.');
}

export function kanonRole(s: KanonSubject): KanonRole {
	if (s.kind !== 'character') return s.kind;
	return s.look ? 'look' : s.persona ? 'persona' : 'character';
}

/** One section of a role: its id and its heading per language */
export interface KanonSectionDef {
	id: string;
	en: string;
	de: string;
}

const S = (id: string, en: string, de: string): KanonSectionDef => ({ id, en, de });
const BACKSTORY = S('backstory', 'Backstory', 'Vorgeschichte');
const LOOK = S('look', 'Look', 'Aussehen');
const RULES = S('rules', 'Rules', 'Regeln');

/** The sections of each role, in their order (KANON.md §4) */
export const KANON_SECTIONS: Record<KanonRole, KanonSectionDef[]> = {
	toon: [S('premise', 'Premise', 'Prämisse'), S('tone', 'Tone', 'Ton'), S('world', 'World rules', 'Weltregeln')],
	character: [BACKSTORY, S('who', 'Who', 'Wer'), LOOK, RULES],
	persona: [S('what', 'What', 'Was'), S('why', 'Why different', 'Warum anders'), LOOK, RULES],
	look: [LOOK, S('when', 'When', 'Wann')],
	place: [BACKSTORY, S('scenery', 'Scenery', 'Szenerie'), RULES]
};

/** The section a generator reads as the visual description of a role (KANON.md §7) */
export const KANON_VISUAL_SECTION: Record<KanonRole, string | null> = { toon: null, character: 'look', persona: 'look', look: 'look', place: 'scenery' };

/** A blank text with every heading of the role in the language (the editor's "Start with the headings") */
export function kanonTemplate(role: KanonRole, locale: string): string {
	return KANON_SECTIONS[role].map((s) => `## ${locale === 'de' ? s.de : s.en}\n\n`).join('');
}

/** A `## ` heading in a text */
export interface KanonHeading {
	/** 1-based line */
	line: number;
	/** the heading as written */
	heading: string;
	/** the section it opens (null: not one of the role's) */
	id: string | null;
}

const HEADING = /^## +(.+?)\s*$/;
const norm = (s: string) => s.trim().toLowerCase();

/** The section id of a heading in any language, null when the role has no such section */
export function kanonSectionId(role: KanonRole, heading: string): string | null {
	const h = norm(heading);
	return KANON_SECTIONS[role].find((s) => norm(s.en) === h || norm(s.de) === h)?.id ?? null;
}

/** Every `## ` heading of a text, in order (the editor's outline) */
export function kanonHeadings(text: string, role: KanonRole): KanonHeading[] {
	const out: KanonHeading[] = [];
	const lines = text.split('\n');
	for (let i = 0; i < lines.length; i++) {
		const m = HEADING.exec(lines[i]);
		if (m) out.push({ line: i + 1, heading: m[1], id: kanonSectionId(role, m[1]) });
	}
	return out;
}

/** The text of one section (without its heading, trimmed); '' when the text has none */
export function kanonSection(text: string, role: KanonRole, id: string): string {
	const lines = text.split('\n');
	const headings = kanonHeadings(text, role);
	const at = headings.findIndex((h) => h.id === id);
	if (at < 0) return '';
	const end = headings[at + 1]?.line ?? lines.length + 1;
	return lines.slice(headings[at].line, end - 1).join('\n').trim();
}

/** Warnings of a text against its role's rules: unknown headings, a section twice, sections out of order */
export function kanonProblems(text: string, role: KanonRole): { line: number; message: string }[] {
	const out: { line: number; message: string }[] = [];
	const order = KANON_SECTIONS[role].map((s) => s.id);
	const seen = new Set<string>();
	let last = -1;
	for (const h of kanonHeadings(text, role)) {
		if (!h.id) {
			out.push({ line: h.line, message: `"## ${h.heading}" is not a ${role} section (${KANON_SECTIONS[role].map((s) => s.en).join(' · ')})` });
			continue;
		}
		if (seen.has(h.id)) out.push({ line: h.line, message: `"## ${h.heading}" comes twice` });
		const i = order.indexOf(h.id);
		if (i < last) out.push({ line: h.line, message: `"## ${h.heading}" is out of order (${KANON_SECTIONS[role].map((s) => s.en).join(' · ')})` });
		last = Math.max(last, i);
		seen.add(h.id);
	}
	return out;
}
