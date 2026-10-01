// A toon's screenplay (entity `script:<toon>:<locale>`, agents/DATA.md "Script"): one big text per language, versioned
// like every entity, for the creator only (no library endpoint, readers never get it). The text follows the format of
// the screenplay editor (C:\Development\screenplay-editor, README "Writing Guide", lib/src/parse.js + utils/regex.js);
// it is stored exactly as typed (whitespace, indentation and trailing spaces are part of the format).

/** Title-page facts of a screenplay (the screenplay editor's `config.json` → `meta`), text in the script's language */
export interface ScriptMeta {
	title?: string;
	author?: string;
	/** e.g. "ENTWURF" / "DRAFT" */
	status?: string;
	/** free text, e.g. "November 2024" */
	date?: string;
	/** e.g. "Basiert auf Liedern" */
	basedOn?: string;
	/** e.g. "Mandy Mozart" */
	basedOnFrom?: string;
}

/** The screenplay of a toon in one language (the source language is the master, translations may be partial) */
export interface ToonScript {
	/** the whole screenplay, plain text in the screenplay format (LF line ends) */
	text: string;
	meta?: ScriptMeta;
}

/** The meta fields in the order the creator shows them */
export const SCRIPT_META_FIELDS = ['title', 'author', 'status', 'date', 'basedOn', 'basedOnFrom'] as const satisfies readonly (keyof ScriptMeta)[];

/** An empty script (a toon without one) */
export function emptyScript(): ToonScript {
	return { text: '' };
}

// ── Outline (read-only, the screenplay editor's rules for headings) ─────────────────────────────────────────────

export interface ScriptOutlineItem {
	kind: 'chapter' | 'scene' | 'song';
	/** 1-based line of the marker / heading */
	line: number;
	/** the chapter / song number, a scene's number as written */
	number: number;
	/** a scene's heading (`INT. KÜCHE - NACHT`); empty for chapters and songs */
	heading: string;
	/** the `# TITLE` line(s) after a chapter or song marker */
	title?: string;
	subtitle?: string;
}

// The screenplay editor matches `# <n>. Kapitel` / `# <n>. Lied` (German terms); translations write their own terms
// (`# 0. Chapter`, `# 1. Song` in the English Cat Pope script), so both are accepted here.
const CHAPTER = /^\s*#\s*(\d+)\.\s*(?:Kapitel|Chapter)\s*$/i;
const SONG = /^\s*#\s*(\d+)\.\s*(?:Lied|Song)\s*$/i;
const TITLE = /^\s*#\s+(.+?)\s*$/;
/** a scene heading starts at column 0 with its number: `12  EXT. ALTERLAA KIRCHENVORPLATZ` */
const SCENE = /^(\d+)\s+(\S.*?)\s*$/;
const PAGE_BREAK = /^---+/;

/**
 * Chapters, scenes and songs of a screenplay with their line numbers (for the creator's outline, click to jump).
 * Same rules as the screenplay editor's parser: a scene counts only inside a chapter; a song's lyrics run until the
 * next chapter, song or page break (`---`); the title (and subtitle) are the `# …` lines right after a marker.
 */
export function scriptOutline(text: string): ScriptOutlineItem[] {
	const lines = text.split('\n');
	const out: ScriptOutlineItem[] = [];
	let inChapter = false;
	let inSong = false;
	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const chapter = CHAPTER.exec(line);
		const song = chapter ? null : SONG.exec(line);
		if (chapter || song) {
			const item: ScriptOutlineItem = { kind: chapter ? 'chapter' : 'song', line: i + 1, number: Number((chapter ?? song)![1]), heading: '' };
			const titles = titlesAfter(lines, i);
			if (titles[0]) item.title = titles[0];
			if (titles[1] && chapter) item.subtitle = titles[1];
			out.push(item);
			inChapter = !!chapter;
			inSong = !!song;
			continue;
		}
		if (inSong) {
			if (PAGE_BREAK.test(line)) inSong = false;
			continue;
		}
		const scene = inChapter ? SCENE.exec(line) : null;
		if (scene) out.push({ kind: 'scene', line: i + 1, number: Number(scene[1]), heading: scene[2] });
	}
	return out;
}

/** Up to two `# …` lines after a marker (blank lines in between are skipped, any other line ends the titles) */
function titlesAfter(lines: string[], marker: number): string[] {
	const titles: string[] = [];
	for (let j = marker + 1; j < lines.length && j <= marker + 4 && titles.length < 2; j++) {
		if (!lines[j].trim()) continue;
		const title = TITLE.exec(lines[j]);
		if (!title || CHAPTER.test(lines[j]) || SONG.test(lines[j])) break;
		titles.push(title[1]);
	}
	return titles;
}
