// Checks config files against the content types of this package (content, audio, visual-novel, enums)
// and across locales (same structure, missing translations, text length). Minimal on the server, thorough here.
import {
	JumpToTargetType,
	Language,
	OnomatopoeiaEffect,
	OnomatopoeiaVariant,
	Renderer,
	Status,
	Trigger,
	TriggerMode
} from './enums.ts';
import { getAt, type Path } from './paths.ts';
import { MOODS, isMood } from './characters.ts';

export type Level = 'error' | 'warning' | 'info';

export interface Problem {
	level: Level;
	/** file path of the doc, e.g. chapter1.json */
	file: string;
	/** null = every locale / cross-locale */
	locale: string | null;
	path: Path;
	message: string;
}

export const LENGTH_WARNING_RATIO = 1.3;

const values = (e: object) => Object.values(e) as string[];
const DIALOG_VARIANTS = ['bubble', 'caption', 'narration', 'sfx'];
const EVENT_TYPES = ['showMessage', 'showChoices', 'addChoice', 'joinScene', 'leaveScene', 'hideMessage', 'showOnomatopeia', 'jumpTo'];
const CHARACTER_VARIANTS = ['avatar', 'full', 'portrait'];
const DATE = /^\d{4}-\d{2}-\d{2}$/;

const isNum = (v: unknown) => typeof v === 'number' && Number.isFinite(v);
const isStr = (v: unknown) => typeof v === 'string';
const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);

/** Text of a dialog / event as one string (arrays joined) */
export function textOf(value: unknown): string {
	if (Array.isArray(value)) return value.join('\n');
	return typeof value === 'string' ? value : '';
}

export interface ValidateContext {
	file: string;
	/** chapter audio config (for slice music/sample ids) */
	audio?: any;
	/** character names the chapter may use: the toon's cast (or the names of the legacy characters.json) */
	characters?: string[];
	/** every character id of the registry (a name in the registry but not in `characters` = "not in the cast") */
	registry?: string[];
	/**
	 * chapter files: the status of this chapter in the season of the same locale (the source of its visibility);
	 * null = the season's sequence doesn't have the chapter; undefined = season not known (no check)
	 */
	seasonStatus?: string | null;
}

class Collector {
	problems: Problem[] = [];
	constructor(
		private file: string,
		private locale: string | null
	) {}
	add(level: Level, path: Path, message: string) {
		this.problems.push({ level, file: this.file, locale: this.locale, path, message });
	}
	error(path: Path, message: string) {
		this.add('error', path, message);
	}
	warn(path: Path, message: string) {
		this.add('warning', path, message);
	}
	info(path: Path, message: string) {
		this.add('info', path, message);
	}
}

function checkRect(c: Collector, rect: unknown, path: Path, heightRequired: boolean) {
	if (!isObj(rect)) {
		c.error(path, 'rect is missing');
		return;
	}
	for (const key of ['x', 'y', 'width'] as const) {
		if (!isNum(rect[key])) c.error([...path, key], `rect.${key} must be a number (% of the slice)`);
	}
	if (heightRequired ? !isNum(rect.height) : rect.height !== undefined && !isNum(rect.height)) {
		c.error([...path, 'height'], 'rect.height must be a number (% of the slice)');
	}
	if (isNum(rect.width) && rect.width <= 0) c.error([...path, 'width'], 'rect.width must be > 0');
	if (isNum(rect.height) && rect.height <= 0) c.error([...path, 'height'], 'rect.height must be > 0');
	if (isNum(rect.x) && (rect.x < -20 || rect.x > 100)) c.warn([...path, 'x'], 'rect.x is outside the slice');
	if (isNum(rect.y) && (rect.y < -100 || rect.y > 100)) c.warn([...path, 'y'], 'rect.y is far outside the slice');
}

function checkDialog(c: Collector, d: any, path: Path) {
	if (!isObj(d)) return c.error(path, 'dialog entry must be an object');
	checkRect(c, d.rect, [...path, 'rect'], false);
	const text = d.text;
	if (!(isStr(text) || (Array.isArray(text) && text.every(isStr)))) c.error([...path, 'text'], 'text must be a string or string[]');
	if (d.id !== undefined && !isStr(d.id)) c.error([...path, 'id'], 'id must be a string');
	if (d.variant !== undefined && !DIALOG_VARIANTS.includes(d.variant)) c.error([...path, 'variant'], `variant must be one of ${DIALOG_VARIANTS.join(', ')}`);
	if (d.frame !== undefined && typeof d.frame !== 'boolean') c.error([...path, 'frame'], 'frame must be true/false');
	if (d.size !== undefined && (!isNum(d.size) || d.size <= 0)) c.error([...path, 'size'], 'size must be a positive number (% of slice width)');
	if (d.align !== undefined && !['left', 'center', 'right'].includes(d.align)) c.error([...path, 'align'], 'align must be left, center or right');
	if (d.valign !== undefined && !['top', 'middle', 'bottom'].includes(d.valign)) c.error([...path, 'valign'], 'valign must be top, middle or bottom');
	if (d.valign !== undefined && d.valign !== 'middle' && d.rect?.height === undefined) c.warn([...path, 'valign'], 'valign only works with rect.height');
	if (d.font !== undefined && !['lettering', 'mono'].includes(d.font)) c.error([...path, 'font'], 'font must be lettering or mono');
	if (d.weight !== undefined && d.weight !== 400 && d.weight !== 700) c.error([...path, 'weight'], 'weight must be 400 or 700');
	if (d.uppercase !== undefined && typeof d.uppercase !== 'boolean') c.error([...path, 'uppercase'], 'uppercase must be true/false');
	if (d.color !== undefined && !isStr(d.color)) c.error([...path, 'color'], 'color must be a CSS color string');
	if (d.rotation !== undefined && !isNum(d.rotation)) c.error([...path, 'rotation'], 'rotation must be a number (degrees)');
	if (d.mask !== undefined && d.mask !== true && !isStr(d.mask)) c.error([...path, 'mask'], 'mask must be true or a CSS color');
	if (d.maskShape !== undefined && !['rect', 'ellipse'].includes(d.maskShape)) c.error([...path, 'maskShape'], 'maskShape must be rect or ellipse');
	if (d.pad !== undefined && !isNum(d.pad)) c.error([...path, 'pad'], 'pad must be a number');
	if (d.frame === false && !['caption', 'narration'].includes(d.variant)) c.warn([...path, 'frame'], 'frame only affects caption / narration');
}

function checkTrigger(c: Collector, trigger: any, path: Path, sample: boolean) {
	if (!isObj(trigger)) {
		if (!sample) c.error(path, 'trigger is missing');
		return;
	}
	if (!values(Trigger).includes(trigger.type)) c.error([...path, 'type'], `trigger.type must be one of ${values(Trigger).join(', ')}`);
	if (trigger.mode !== undefined && !values(TriggerMode).includes(trigger.mode)) c.error([...path, 'mode'], 'trigger.mode must be mono or poly');
	if ((trigger.type === Trigger.CLICK || trigger.type === Trigger.HOVER) && sample) {
		checkRect(c, trigger.rect, [...path, 'rect'], true);
	} else if (trigger.rect !== undefined) {
		checkRect(c, trigger.rect, [...path, 'rect'], true);
	}
}

function checkTarget(c: Collector, target: any, path: Path) {
	if (!isObj(target)) return c.error(path, 'target is missing');
	if (!values(JumpToTargetType).includes(target.type)) c.error([...path, 'type'], `target.type must be one of ${values(JumpToTargetType).join(', ')}`);
	if (target.value === undefined || target.value === '') c.error([...path, 'value'], 'target.value is missing');
}

function unknownCharacter(name: string, ctx: ValidateContext) {
	return ctx.registry?.includes(name) ? `character "${name}" is not in the toon's cast` : `unknown character "${name}"`;
}

export function checkEvent(c: Collector, e: any, path: Path, ctx: ValidateContext, sliceEvent: boolean) {
	if (!isObj(e)) return c.error(path, 'event must be an object');
	if (!EVENT_TYPES.includes(e.type)) {
		c.warn([...path, 'type'], `unknown event type "${e.type}" – the reader skips it`);
		return;
	}
	const p = e.params;
	const pp = [...path, 'params'];
	switch (e.type) {
		case 'showMessage':
			if (!isObj(p) || !isStr(p.text)) c.error([...pp, 'text'], 'showMessage needs params.text');
			if (isObj(p) && p.character && ctx.characters && !ctx.characters.includes(p.character)) {
				c.warn([...pp, 'character'], unknownCharacter(p.character, ctx));
			}
			break;
		case 'addChoice':
			if (!isObj(p) || !isStr(p.label)) c.error([...pp, 'label'], 'addChoice needs params.label');
			if (isObj(p?.onSelect?.jumpTo)) checkTarget(c, p.onSelect.jumpTo.target, [...pp, 'onSelect', 'jumpTo', 'target']);
			break;
		case 'jumpTo':
			if (!isObj(p)) c.error(pp, 'jumpTo needs params.target');
			else checkTarget(c, p.target, [...pp, 'target']);
			break;
		case 'joinScene':
		case 'leaveScene':
			if (!isObj(p) || !Array.isArray(p.characters)) c.error([...pp, 'characters'], `${e.type} needs params.characters[]`);
			else
				p.characters.forEach((ch: any, i: number) => {
					if (!isObj(ch) || !isStr(ch.name)) c.error([...pp, 'characters', i, 'name'], 'character needs a name');
					else if (ctx.characters && !ctx.characters.includes(ch.name)) c.warn([...pp, 'characters', i, 'name'], unknownCharacter(ch.name, ctx));
					if (isObj(ch) && ch.mood !== undefined && ch.mood !== '' && !isMood(ch.mood)) {
						c.warn([...pp, 'characters', i, 'mood'], `mood "${ch.mood}" is not one of ${MOODS.join(', ')} – the reader shows neutral`);
					}
					if (ch?.position?.variant && !CHARACTER_VARIANTS.includes(ch.position.variant)) {
						c.error([...pp, 'characters', i, 'position'], 'position.variant must be avatar, full or portrait');
					}
				});
			break;
		case 'showOnomatopeia':
			if (!isObj(p) || !(isStr(p.text) || Array.isArray(p.text))) c.error([...pp, 'text'], 'showOnomatopeia needs params.text');
			if (isObj(p)) {
				if (p.variant !== undefined && !values(OnomatopoeiaVariant).includes(p.variant)) c.error([...pp, 'variant'], `variant must be one of ${values(OnomatopoeiaVariant).join(', ')}`);
				if (p.effect !== undefined && !values(OnomatopoeiaEffect).includes(p.effect)) c.error([...pp, 'effect'], `effect must be one of ${values(OnomatopoeiaEffect).join(', ')}`);
				if (p.duration !== undefined && (!isNum(p.duration) || p.duration > 5000)) c.warn([...pp, 'duration'], 'duration is capped at 5000 ms by the reader');
				if (p.delay !== undefined && (!isNum(p.delay) || p.delay > 5000)) c.warn([...pp, 'delay'], 'delay is capped at 5000 ms by the reader');
			}
			break;
	}
	if (sliceEvent) checkTrigger(c, e.trigger, [...path, 'trigger'], false);
}

function checkSlice(c: Collector, s: any, path: Path, ctx: ValidateContext) {
	if (!isObj(s)) return c.error(path, 'slice must be an object');
	if (!isStr(s.image) || !s.image) c.error([...path, 'image'], 'slice.image is missing');
	if (s.dialog !== undefined) {
		if (!Array.isArray(s.dialog)) c.error([...path, 'dialog'], 'dialog must be an array');
		else s.dialog.forEach((d: any, i: number) => checkDialog(c, d, [...path, 'dialog', i]));
	}
	const audio = s.audio;
	if (audio !== undefined) {
		if (!isObj(audio)) c.error([...path, 'audio'], 'audio must be an object');
		else {
			const trackIds: string[] = (ctx.audio?.music?.tracks ?? []).map((t: any) => t?.id);
			(audio.music?.tracks ?? []).forEach((t: any, i: number) => {
				const tp = [...path, 'audio', 'music', 'tracks', i];
				if (!trackIds.includes(t?.id)) c.error([...tp, 'id'], `music track "${t?.id}" is not in the chapter's audio.music.tracks`);
				if (!isNum(t?.volume) || t.volume < 0 || t.volume > 1) c.error([...tp, 'volume'], 'volume must be 0…1');
			});
			const sampleIds: string[] = (ctx.audio?.sampleLibrary ?? []).map((t: any) => t?.id);
			const checkSamples = (list: any, base: Path) =>
				(Array.isArray(list) ? list : []).forEach((smp: any, i: number) => {
					const sp = [...base, i];
					if (!sampleIds.includes(smp?.id)) c.error([...sp, 'id'], `sample "${smp?.id}" is not in the chapter's sampleLibrary`);
					if (smp?.volume !== undefined && (!isNum(smp.volume) || smp.volume < 0 || smp.volume > 1)) c.error([...sp, 'volume'], 'volume must be 0…1');
					checkTrigger(c, smp?.trigger, [...sp, 'trigger'], true);
				});
			checkSamples(audio.audio?.samples, [...path, 'audio', 'audio', 'samples']);
			if (Array.isArray(audio.samples)) {
				c.warn([...path, 'audio', 'samples'], 'samples at audio.samples are never played – the reader reads audio.audio.samples (move them)');
				checkSamples(audio.samples, [...path, 'audio', 'samples']);
			}
		}
	}
	if (s.events !== undefined) {
		if (!Array.isArray(s.events)) c.error([...path, 'events'], 'events must be an array');
		else s.events.forEach((e: any, i: number) => checkEvent(c, e, [...path, 'events', i], ctx, true));
	}
}

export function checkGroup(c: Collector, g: any, path: Path, ctx: ValidateContext) {
	if (!isObj(g)) return c.error(path, 'group must be an object');
	if (g.ref) {
		if (!isStr(g.ref.src) || !/^[a-z0-9_-]+(\/[a-z0-9_-]+)?\.json$/.test(g.ref.src)) {
			c.error([...path, 'ref', 'src'], 'ref.src must be a file like chapter2/scene-14.json');
		}
		return;
	}
	if (!g.renderer || !values(Renderer).includes(g.renderer.type)) {
		c.error([...path, 'renderer', 'type'], 'renderer.type must be vertical or horizontal (the reader shows "Renderer is missing")');
	}
	if (!g.label) c.warn([...path, 'label'], 'group has no label (?label= links and jumpTo need one)');
	if (!Array.isArray(g.slices) && !Array.isArray(g.events)) c.error(path, 'group needs slices or events');
	if (Array.isArray(g.slices)) {
		if (!g.slices.length && !g.events) c.warn([...path, 'slices'], 'group has no slices');
		g.slices.forEach((s: any, i: number) => checkSlice(c, s, [...path, 'slices', i], ctx));
	}
	if (g.events !== undefined) {
		if (!Array.isArray(g.events)) c.error([...path, 'events'], 'events must be an array');
		else g.events.forEach((e: any, i: number) => checkEvent(c, e, [...path, 'events', i], ctx, false));
	}
}

function checkChapter(c: Collector, ch: any, locale: string, ctx: ValidateContext) {
	if (!isObj(ch)) return c.error([], 'chapter file must be an object');
	for (const key of ['title', 'author', 'description', 'thumbnail'] as const) {
		if (!isStr(ch[key])) c.error([key], `${key} must be a string`);
	}
	if (isStr(ch.title) && !ch.title.trim()) c.warn(['title'], 'title is empty');
	if (!Number.isInteger(ch.number)) c.error(['number'], 'number must be an integer');
	if (!values(Language).includes(ch.language)) c.error(['language'], `language must be one of ${values(Language).join(', ')}`);
	else if (ch.language !== locale) c.warn(['language'], `language is "${ch.language}" in the ${locale} file`);
	if (!isStr(ch.release) || !DATE.test(ch.release)) c.error(['release'], 'release must be a date YYYY-MM-DD');
	// visibility comes from the season (per locale); the chapter's own status is a deprecated mirror of it
	if (ch.status !== undefined && !values(Status).includes(ch.status)) c.error(['status'], `status must be one of ${values(Status).join(', ')}`);
	// a chapter always belongs to a season: without an entry the reader never lists it
	if (ctx.seasonStatus === null) c.error(['status'], `the ${locale} season has no entry for this chapter – every chapter must belong to a season`);
	else if (ctx.seasonStatus !== undefined && ch.status !== undefined && ch.status !== ctx.seasonStatus)
		c.warn(['status'], `status "${ch.status}" differs from the season ("${ctx.seasonStatus}"), which is the source – saving aligns it`);
	if (ch.published !== undefined) c.info(['published'], 'published is no longer used (the season status is the source) – saving removes it');
	if (ch.keyvisual !== undefined) {
		if (isStr(ch.keyvisual)) c.error(['keyvisual'], 'keyvisual must be an object { image, video? } (string form is outdated)');
		else if (!isObj(ch.keyvisual) || !isStr(ch.keyvisual.image)) c.error(['keyvisual', 'image'], 'keyvisual.image is missing');
	}
	if (ch.audio !== undefined) {
		const music = ch.audio?.music;
		const ids = (music?.tracks ?? []).map((t: any) => t?.id);
		if (new Set(ids).size !== ids.length) c.error(['audio', 'music', 'tracks'], 'music track ids must be unique');
		(music?.tracks ?? []).forEach((t: any, i: number) => {
			if (!isStr(t?.id) || !isStr(t?.path)) c.error(['audio', 'music', 'tracks', i], 'track needs id and path');
		});
		if (music?.defaultTrack && !ids.includes(music.defaultTrack)) c.error(['audio', 'music', 'defaultTrack'], 'defaultTrack is not one of the tracks');
		const sids = (ch.audio?.sampleLibrary ?? []).map((t: any) => t?.id);
		if (new Set(sids).size !== sids.length) c.error(['audio', 'sampleLibrary'], 'sample ids must be unique');
	}
	if (!Array.isArray(ch.groups) || !ch.groups.length) c.error(['groups'], 'chapter needs at least one group');
	else ch.groups.forEach((g: any, i: number) => checkGroup(c, g, ['groups', i], { ...ctx, audio: ch.audio }));
}

function checkStringList(c: Collector, value: unknown, path: Path, required = false) {
	if (value === undefined) {
		if (required) c.error(path, `${path.join('.')} must be a list of names`);
		return;
	}
	if (!Array.isArray(value) || value.some((v) => !isStr(v))) c.error(path, `${path.join('.')} must be a list of names`);
	else if (value.some((v) => !v.trim())) c.warn(path, `${path.join('.')} has an empty name`);
}

function checkLanguage(c: Collector, value: unknown, locale: string, required: boolean) {
	if (value === undefined && !required) return;
	if (!values(Language).includes(value as string)) c.error(['language'], `language must be one of ${values(Language).join(', ')}`);
	else if (value !== locale) c.warn(['language'], `language is "${value}" in the ${locale} file`);
}

function checkImagePath(c: Collector, value: unknown, path: Path, required = false) {
	if (value === undefined || value === null || value === '') {
		if (required) c.warn(path, `${path.join('.')} is empty`);
		return;
	}
	if (!isStr(value)) c.error(path, `${path.join('.')} must be a path on the CDN`);
	else if (!/\.(jpe?g|png|webp|gif|svg)$/i.test(value)) c.warn(path, `${path.join('.')} is not an image path (.jpg, .png, .webp, .gif, .svg)`);
}

/** season1.json: meta (Season type) + the sequence (the chapters' visibility) */
function checkSeason(c: Collector, s: any, locale: string) {
	if (!isObj(s)) return c.error([], 'season file must be an object');
	if (!isStr(s.title) || !s.title.trim()) c.warn(['title'], 'title is empty');
	if (s.description !== undefined && !isStr(s.description)) c.error(['description'], 'description must be a string');
	else if (!s.description?.trim()) c.warn(['description'], 'description (plot) is empty');
	if (s.number !== undefined && !Number.isInteger(s.number)) c.error(['number'], 'number must be an integer');
	if (s.release !== undefined && (!isStr(s.release) || !DATE.test(s.release))) c.error(['release'], 'release must be a date YYYY-MM-DD');
	checkLanguage(c, s.language, locale, false);
	checkStringList(c, s.authors, ['authors']);
	if (s.cover !== undefined) {
		if (!isObj(s.cover)) c.error(['cover'], 'cover must be { square?, portrait? }');
		else for (const key of ['square', 'portrait']) checkImagePath(c, s.cover[key], ['cover', key]);
	}
	if (s.credits !== undefined) {
		if (!isObj(s.credits)) c.error(['credits'], 'credits must be { artist?, writer?, music?, sound? }');
		else {
			for (const key of ['artist', 'music', 'sound']) checkStringList(c, s.credits[key], ['credits', key]);
			if (s.credits.writer !== undefined && !isStr(s.credits.writer)) c.error(['credits', 'writer'], 'credits.writer must be a name');
		}
	}
	if (!Array.isArray(s.sequence)) c.error(['sequence'], 'season needs a sequence');
	else {
		const numbers = new Set<number>();
		s.sequence.forEach((item: any, i: number) => {
			if (item?.type !== 'chapter' || !Number.isInteger(item?.number)) c.error(['sequence', i], 'sequence item needs type "chapter" and a number');
			else if (numbers.has(item.number)) c.error(['sequence', i], `chapter ${item.number} is in the sequence twice`);
			else numbers.add(item.number);
			if (!values(Status).includes(item?.status)) c.error(['sequence', i, 'status'], 'status must be published, draft or coming-soon');
		});
	}
}

/** toon.json (Toon type): what the reader shows on the toon page, the map and the download page */
function checkToon(c: Collector, t: any, locale: string) {
	if (!isObj(t)) return c.error([], 'toon file must be an object');
	if (!isStr(t.id) || !t.id) c.error(['id'], 'id must be the toon id');
	if (!isStr(t.title) || !t.title.trim()) c.warn(['title'], 'title is empty');
	if (!isStr(t.description) || !t.description.trim()) c.warn(['description'], 'description is empty');
	checkLanguage(c, t.language, locale, true);
	checkStringList(c, t.authors, ['authors'], true);
	if (!Number.isInteger(t.chapters)) c.error(['chapters'], 'chapters must be an integer');
	if (!isStr(t.release) || !DATE.test(t.release)) c.error(['release'], 'release must be a date YYYY-MM-DD');
	if (!values(Status).includes(t.status)) c.error(['status'], `status must be one of ${values(Status).join(', ')}`);
	checkImagePath(c, t.thumbnail, ['thumbnail'], true);
	if (!isObj(t.cover)) c.error(['cover'], 'cover must be { square?, portrait? }');
	else {
		checkImagePath(c, t.cover.square, ['cover', 'square'], true);
		checkImagePath(c, t.cover.portrait, ['cover', 'portrait']);
	}
	if (!isObj(t.map)) c.error(['map'], 'map must be { layers, width, height, center }');
	else {
		checkImagePath(c, t.map.layers?.terrain, ['map', 'layers', 'terrain'], true);
		for (const key of ['width', 'height'] as const) if (!isNum(t.map[key]) || t.map[key] <= 0) c.warn(['map', key], `map.${key} must be a positive number (px)`);
		for (const key of ['x', 'y'] as const) if (!isNum(t.map.center?.[key])) c.error(['map', 'center', key], `map.center.${key} must be a number (px)`);
	}
	if (!isObj(t.download)) c.error(['download'], 'download must be { complete, music }');
	else {
		if (t.download.complete !== null && !isStr(t.download.complete)) c.error(['download', 'complete'], 'download.complete must be a path or null');
		if (!isStr(t.download.music)) c.error(['download', 'music'], 'download.music must be a path');
	}
}

/** Problems of one locale file */
export function validateFile(kind: string, data: any, locale: string, ctx: ValidateContext): Problem[] {
	const c = new Collector(ctx.file, locale);
	if (data === null || data === undefined) {
		c.error([], `no ${locale} file`);
		return c.problems;
	}
	if (kind === 'chapter') checkChapter(c, data, locale, ctx);
	else if (kind === 'group') checkGroup(c, data, [], ctx);
	else if (kind === 'season') checkSeason(c, data, locale);
	else if (kind === 'toon') checkToon(c, data, locale);
	return c.problems;
}

/** Walks every slice of a chapter/group file */
export function forEachSlice(kind: string, data: any, fn: (slice: any, path: Path) => void) {
	const groups: [any, Path][] = kind === 'chapter' ? (data?.groups ?? []).map((g: any, i: number) => [g, ['groups', i]]) : [[data, []]];
	for (const [g, gp] of groups) (g?.slices ?? []).forEach((s: any, i: number) => fn(s, [...gp, 'slices', i]));
}

/** Structure + translations across the locale files of one doc */
export function validateLocales(kind: string, file: string, files: Record<string, any>, primary: string): Problem[] {
	const problems: Problem[] = [];
	const add = (level: Level, locale: string | null, path: Path, message: string) =>
		problems.push({ level, file, locale, path, message });
	const source = files[primary];
	if (!source) return problems;
	for (const [locale, data] of Object.entries(files)) {
		if (locale === primary || !data) continue;
		if (kind === 'chapter') {
			const a = source.groups ?? [];
			const b = data.groups ?? [];
			if (a.length !== b.length) add('error', locale, ['groups'], `${locale} has ${b.length} groups, ${primary} has ${a.length}`);
			a.forEach((g: any, i: number) => {
				if (!b[i]) return;
				if ((g.ref?.src ?? null) !== (b[i].ref?.src ?? null)) add('warning', locale, ['groups', i, 'ref'], `group ${i + 1}: ref differs between ${primary} and ${locale}`);
				const sa = g.slices ?? [];
				const sb = b[i].slices ?? [];
				if (sa.length !== sb.length) add('error', locale, ['groups', i, 'slices'], `group ${i + 1}: ${locale} has ${sb.length} slices, ${primary} has ${sa.length}`);
			});
		} else if (kind === 'season' || kind === 'toon') {
			for (const key of ['title', 'description']) {
				const src = textOf(source[key]).trim();
				const tr = textOf(data[key]).trim();
				if (src && !tr) add('warning', locale, [key], `missing ${locale} ${key}`);
				else if (src && tr === src) add('info', locale, [key], `${locale} ${key} is the same as ${primary} (not translated?)`);
			}
			if (kind === 'season') {
				const numbers = (file: any) => (file?.sequence ?? []).map((i: any) => Number(i?.number)).sort((a: number, b: number) => a - b).join(',');
				if (numbers(source) !== numbers(data)) add('warning', locale, ['sequence'], `${locale} season lists chapters ${numbers(data) || '–'}, ${primary} lists ${numbers(source) || '–'}`);
			}
		} else if (kind === 'group') {
			const sa = source.slices ?? [];
			const sb = data.slices ?? [];
			if (sa.length !== sb.length) add('error', locale, ['slices'], `${locale} has ${sb.length} slices, ${primary} has ${sa.length}`);
			if ((source.events ?? []).length !== (data.events ?? []).length) add('warning', locale, ['events'], `${locale} has ${(data.events ?? []).length} events, ${primary} has ${(source.events ?? []).length}`);
		}
		forEachSlice(kind, source, (slice, path) => {
			const other = getAt(data, path);
			if (!other) return;
			if (other.image !== slice.image) add('info', locale, [...path, 'image'], `${locale} uses another image (override)`);
			const da: any[] = slice.dialog ?? [];
			const db: any[] = other.dialog ?? [];
			if (da.length !== db.length) {
				add('error', locale, [...path, 'dialog'], `${locale} has ${db.length} text boxes, ${primary} has ${da.length}`);
				return;
			}
			da.forEach((d, i) => {
				const o = db[i];
				const dp = [...path, 'dialog', i];
				if ((d?.id ?? null) !== (o?.id ?? null)) add('error', locale, [...dp, 'id'], `text box ${i + 1}: id ${o?.id} ≠ ${d?.id} (order differs)`);
				const src = textOf(d?.text).trim();
				const tr = textOf(o?.text).trim();
				if (src && !tr) add('warning', locale, [...dp, 'text'], `missing ${locale} text for "${src.slice(0, 40)}"`);
				else if (src && tr === src && locale !== primary) add('info', locale, [...dp, 'text'], `${locale} text is the same as ${primary} (not translated?)`);
				else if (src && tr.length > src.length * LENGTH_WARNING_RATIO) {
					add('warning', locale, [...dp, 'text'], `${locale} text is ${Math.round((tr.length / src.length) * 100)}% of ${primary} (> 130%) – check the fit`);
				}
			});
		});
	}
	return problems;
}
