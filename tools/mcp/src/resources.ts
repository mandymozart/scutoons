// The format rules an agent reads before editing (MCP resources `scutoons://rules/*`). Generated from the source files
// at start-up – the doc comments, types and constants the apps use – so they never drift from the code. Short on
// purpose: the agent reads the full types where it needs them.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
	DEFAULT_SLICE,
	FRAME_PRESETS,
	FRAME_RATIO_MAX,
	FRAME_RATIO_MIN,
	IMAGE_QUALITY_DENSITIES,
	VIDEO_QUALITY_LABELS
} from '@scutoons/content';
import { TEXT_PRESETS } from '../../../creator/src/lib/editor/text-presets.ts';
import { ROOT } from './config.ts';

const source = (file: string) => readFileSync(resolve(ROOT, file), 'utf8');

/** The leading `//` comment block of a file */
function headerComment(file: string): string {
	const lines: string[] = [];
	for (const line of source(file).split(/\r?\n/)) {
		if (!line.startsWith('//')) break;
		lines.push(line.replace(/^\/\/ ?/, ''));
	}
	return lines.join('\n').trim();
}

/** The `/** … *\/` comment right before `<declaration>` (e.g. `function checkVnFlow`), without the stars */
function docComment(file: string, declaration: string): string {
	const text = source(file);
	const at = text.indexOf(declaration);
	if (at < 0) return '';
	const end = text.lastIndexOf('*/', at);
	const start = text.lastIndexOf('/**', end);
	if (start < 0 || text.slice(end + 2, at).trim().replace(/^export\s*/, '') !== '') return '';
	return text
		.slice(start + 3, end)
		.split(/\r?\n/)
		.map((l) => l.replace(/^\s*\* ?/, ''))
		.join('\n')
		.trim();
}

/** A balanced `{ … }` block starting at `<declaration>` (an interface / type / function), verbatim */
function block(file: string, declaration: string): string {
	const text = source(file);
	const at = text.indexOf(declaration);
	if (at < 0) return '';
	let depth = 0;
	for (let i = text.indexOf('{', at); i < text.length; i++) {
		if (text[i] === '{') depth++;
		else if (text[i] === '}' && --depth === 0) return text.slice(at, i + 1);
	}
	return '';
}

/** A markdown section (from `## <title>` to the next `## `) */
function section(file: string, title: string): string {
	const text = source(file);
	const at = text.indexOf(`## ${title}`);
	if (at < 0) return '';
	const next = text.indexOf('\n## ', at + 3);
	return text.slice(at, next < 0 ? undefined : next).trim();
}

const code = (lang: string, text: string) => '```' + lang + '\n' + text.trim() + '\n```';

export interface RuleResource {
	name: string;
	uri: string;
	title: string;
	description: string;
	text: () => string;
}

export const RULES: RuleResource[] = [
	{
		name: 'workflow',
		uri: 'scutoons://rules/workflow',
		title: 'How to edit SCUTOONS content (read first)',
		description: 'The editing loop, the safety model and where the other rules are',
		text: () =>
			[
				'# Editing SCUTOONS content',
				'',
				'You act with the rights of the signed-in creator; the API checks every call.',
				'',
				'## Finding things (the whole universe)',
				'',
				'- `list_toons`: every toon, its languages and its SOURCE language (the first, e.g. catpope: de).',
				'- `list_entities`: every stored document of a toon (or toon "" for the platform-wide ones: characters,',
				'  moods, the world map, people, roles, events) – the screenplay is `script:<toon>:<locale>`.',
				'- `search_content`: full-text search in a toon\'s documents – story, song titles, lyrics, names, dialog.',
				'  When the creator mentions something ("the song X in the script", "the scene where …"), SEARCH before',
				'  saying it does not exist; search without a locale filter.',
				'- The SOURCE language holds the complete content. Translations can be older or incomplete – read and',
				'  search the source language first, then compare the translations.',
				'- Long texts (the screenplay ~70k characters): search, then read_entity with path ["text"] and `lines`.',
				'',
				'## Editing',
				'',
				'1. Find the document: `list_chapters` (groups, their labels and script points), `list_entities`, `search_content`.',
				'2. `read_entity` (draft) → note `version.id`: it is the `base` of your save.',
				'3. Change it: `save_draft` with `ops` (set / insert / remove at a path) or the full `content`.',
				'   It validates first (same checks as the creator) and refuses errors; warnings are returned.',
				'4. A 409 conflict = someone saved a newer version: read again, reapply, save with the new base.',
				'5. Every save is a NEW draft version (append-only). Nothing is overwritten; `list_versions` +',
				'   `restore_version` bring any older version back as a new draft.',
				'6. Readers see only published versions. Publishing is off unless the server runs with',
				'   SCUTOONS_MCP_PUBLISH=1, and then each `publish_entity` call needs `confirm: true` – ask the owner first.',
				'',
				'## Languages',
				'',
				'Chapters, scenes and seasons are ONE language-neutral document each (`chapter:catpope:2`,',
				'`scene:catpope:2:scene-14`, `season:catpope:1` – no locale). Structure, media paths and geometry exist once;',
				'every TEXT is a locale map `{ "de": "…", "en": "…" }`: chapter / season `title`, `description`, `release` (a date',
				'per language), a text box `text` (string or string[]), showMessage `params.text`, addChoice `params.label`, and',
				'the season `sequence[].status` (visibility per language). Nothing else has a language.',
				'- A new text box / message / choice: give it a text in the source language at least; a missing translation',
				'  shows the source text to readers (and is a warning).',
				'- A translation: set only that language, e.g. `set` at ["groups", 2, "slices", 4, "dialog", 0, "text", "en"].',
				'- A blank text counts as missing. Never add a per-language copy of structure.',
				'The toon settings are the toon entry `toon:<toon>` (one document): `title`, `description`, `release` and',
				'`languageStatus` (the visibility per language) are locale maps there, the rest (authors, tile, map …) exists',
				'once. The old `toon-settings:<toon>:<locale>` documents are read only (legacy).',
				'Only the screenplay (`script:<toon>:<locale>`) is stored per language: save each locale on its own key.',
				'',
				'Rules: scutoons://rules/entity-keys, scutoons://rules/vn-events, scutoons://rules/flow,',
				'scutoons://rules/text-presets, scutoons://rules/slices.'
			].join('\n')
	},
	{
		name: 'entity-keys',
		uri: 'scutoons://rules/entity-keys',
		title: 'Entity keys',
		description: 'How every document is addressed (shared/content/src/entities.ts)',
		text: () =>
			[
				'# Entity keys',
				'',
				headerComment('shared/content/src/entities.ts'),
				'',
				'A chapter group either holds its slices inline or references a scene: `{ "ref": { "src": "chapter2/scene-14.json" } }`',
				'= the entity `scene:<toon>:2:scene-14` (one language-neutral document). Edit the scene entity for such a group.'
			].join('\n')
	},
	{
		name: 'vn-events',
		uri: 'scutoons://rules/vn-events',
		title: 'Visual novel events',
		description: 'The VN event format: story flow (label / divert / if / setVariable / choices / jumpTo anyOf) and the types',
		text: () =>
			[
				'# Visual novel events',
				'',
				'A group has a script (`group.events`, runs when the group opens); slices have events too (`slice.events`, each',
				'with a `trigger`), and placed items with `action: "events"` run `item.events` on tap. All use the same events.',
				'',
				section('viewer/src/lib/visual-novel/README.md', 'Story flow').replace(/^## /, '### '),
				'',
				'Choice actions (`addChoice.params.onSelect`: one action or a list, run in order): setVariable / variable',
				'actions first, then a `jumpTo` (navigates) or a `divert { to }` (continues the waiting script).',
				'',
				'### Helpers (shared/content/src/vn-script.ts)',
				...['function testCondition', 'function scriptLabels', 'function pickJumpLabel', 'function formatClock', 'function interpolateText']
					.map((d) => `- \`${d.replace('function ', '')}\`: ${docComment('shared/content/src/vn-script.ts', d).replace(/\n/g, ' ')}`),
				'',
				'### Types (shared/content/src/visual-novel.ts)',
				code('ts', source('shared/content/src/visual-novel.ts'))
			].join('\n')
	},
	{
		name: 'flow',
		uri: 'scutoons://rules/flow',
		title: 'Flow rules the validator checks',
		description: 'Script points and the VN flow per group (validate.ts checkScriptPoints / checkVnFlow)',
		text: () =>
			[
				'# Flow rules (shared/content/src/validate.ts)',
				'',
				'## Script points (errors)',
				docComment('shared/content/src/validate.ts', 'function checkScriptPoints'),
				'',
				'## VN flow (warnings)',
				docComment('shared/content/src/validate.ts', 'function checkVnFlow'),
				'',
				'A `divert` / `jumpTo` without `if` leaves the script: choices added before it and not shown are lost.',
				'Use `validate` before saving; `save_draft` refuses errors.'
			].join('\n')
	},
	{
		name: 'text-presets',
		uri: 'scutoons://rules/text-presets',
		title: 'Text presets',
		description: 'The five looks of a text on a slice (creator/src/lib/editor/text-presets.ts)',
		text: () =>
			[
				'# Text presets',
				'',
				docComment('creator/src/lib/editor/text-presets.ts', 'export type TextPresetId'),
				'',
				'A text is a `Dialog` in `slice.dialog[]`: `{ id, text, rect: { x, y, width, height? } (% of the slice), …look }`.',
				'Use exactly one preset\'s look fields (below) for a new text. In a language-neutral chapter / scene the text',
				'is a locale map `{ "de": "…", "en": "…" }` (rect and look exist once for every language).',
				'',
				code('json', TEXT_PRESETS.map((p) => JSON.stringify(p)).join('\n')),
				'',
				'Dialog type (shared/content/src/content.ts):',
				code('ts', block('shared/content/src/content.ts', 'export interface Dialog'))
			].join('\n')
	},
	{
		name: 'slices',
		uri: 'scutoons://rules/slices',
		title: 'Slices, sizes and media variants',
		description: 'Slice size, frame ratios, image / video quality variants (shared/content/src/content.ts)',
		text: () =>
			[
				'# Slices and media',
				'',
				docComment('shared/content/src/content.ts', 'export const DEFAULT_SLICE'),
				'',
				`- standard slice: ${DEFAULT_SLICE.width} × ${DEFAULT_SLICE.height} px (width ${DEFAULT_SLICE.width} always; height may differ)`,
				`- image variants (\`imageVariants\`): ${Object.entries(IMAGE_QUALITY_DENSITIES).map(([l, d]) => `${l} = ${d}x (${DEFAULT_SLICE.width * d} px)`).join(', ')}`,
				`- video variants (\`video.variants\`): ${Object.entries(VIDEO_QUALITY_LABELS).map(([l, v]) => `${l} = ${v}`).join(', ')}`,
				`- video frame presets: ${FRAME_PRESETS.map((p) => `${p.id} ${p.ratio}`).join(', ')} (allowed ${1 / FRAME_RATIO_MIN}:1 … 1:${FRAME_RATIO_MAX})`,
				'- media paths are relative to the toon folder on the CDN and language-independent (no locale in a path);',
				'  find existing files with `search_assets`.',
				'',
				code('ts', block('shared/content/src/content.ts', 'export interface Slice {'))
			].join('\n')
	}
];
