// Checks config files against the content types of this package (content, audio, visual-novel, enums)
// and across locales (same structure, missing translations, text length). Minimal on the server, thorough here.
import {
	JumpToTargetType,
	Language,
	OnomatopoeiaEffect,
	Renderer,
	Status,
	Trigger,
	TriggerMode
} from './enums.ts';
import { getAt, type Path } from './paths.ts';
import { hasVisualNovel, linksToNext } from './chains.ts';
import { LEGACY_CHARACTER_IDS, REGULAR_PERSONA, resolveCharacterId } from './characters.ts';
import {
	DEFAULT_SLICE,
	DIALOG_SHAPES,
	DIALOG_TAILS,
	SLICE_ITEM_ACTIONS,
	TOON_NEUTRAL_FIELDS,
	VIEWPORT_FITS,
	frameSliceSize,
	hasToonTexts,
	isFrameRatioAllowed,
	parseFrameRatio,
	type ToonIndex
} from './content.ts';
import { checkWorldMap } from './map.ts';
import { SCRIPT_META_FIELDS } from './script.ts';
import { mediaLocaleMessage, mediaPathsWithLocale } from './media.ts';
import { isMediaQuality } from './media-quality.ts';
import { checkRelatedItems } from './albums.ts';
import { placementRespawnProblem, type ItemRespawn } from './respawn.ts';
import { COVER_ALIGNS, COVER_COLORS, COVER_FORMATS, COVER_LIMITS, COVER_TEXT_SOURCES, coverLetteringOn } from './cover.ts';
import { resolveChapter, resolveGroup, resolveSeason, resolveToon, type ChapterDoc, type GroupDoc, type NeutralKind, type NeutralLanguages, type SeasonDoc } from './neutral.ts';
import { forEachText, type TextLeafKind } from './neutralize.ts';

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
const DIALOG_VARIANTS = ['bubble', 'caption', 'narration'];
const EVENT_TYPES = ['showMessage', 'showChoices', 'addChoice', 'joinScene', 'leaveScene', 'hideMessage', 'showOnomatopeia', 'jumpTo', 'label', 'divert', 'setVariable'];
const CONDITION_OPS = ['==', '!=', '<', '<=', '>', '>=', 'set', 'unset'];
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
	/** character names of the legacy characters.json (when the toon has no cast in the store) */
	characters?: string[];
	/** the season's cast: character id → persona ids (`castIndex` in characters.ts) */
	cast?: Record<string, string[]>;
	/** the registry: character id → persona → { defaultLook, looks: look → moods with a portrait } (`expressionIndex`) */
	expressions?: Record<string, Record<string, { defaultLook: string; looks: Record<string, string[]> }>>;
	/** the mood vocabulary's ids */
	moods?: string[];
	/**
	 * chapter files: the status of this chapter in the season of the same locale (the source of its visibility);
	 * null = the season's sequence doesn't have the chapter; undefined = season not known (no check)
	 */
	seasonStatus?: string | null;
	/** the item catalog's ids (`related_items` of seasons / chapters are checked against them; none = no check) */
	items?: string[];
	/** the catalog's respawn rules by item id (null = never reappears): placed items' `respawn` is checked against them */
	itemRespawn?: Record<string, ItemRespawn | null>;
	/**
	 * chapter files: a `ref` group → its scene (the creator's loaded scene document), so the chapter's linked groups
	 * (chains.ts) are checked with the scenes' `continues`; none / null = the ref group as it is
	 */
	resolveGroup?: (group: any) => any;
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
	if (d.stroke !== undefined && !isStr(d.stroke)) c.error([...path, 'stroke'], 'stroke must be a CSS color string');
	if (d.strokeWidth !== undefined && (!isNum(d.strokeWidth) || d.strokeWidth < 0)) c.error([...path, 'strokeWidth'], 'strokeWidth must be a number ≥ 0 (em)');
	if (d.letterSpacing !== undefined && (!isNum(d.letterSpacing) || d.letterSpacing < -0.5 || d.letterSpacing > 2)) c.error([...path, 'letterSpacing'], 'letterSpacing must be a number of em between -0.5 and 2');
	if (d.tail !== undefined && !(DIALOG_TAILS as readonly string[]).includes(d.tail)) c.error([...path, 'tail'], `tail must be one of ${DIALOG_TAILS.join(', ')}`);
	if (d.shape !== undefined && !(DIALOG_SHAPES as readonly string[]).includes(d.shape)) c.error([...path, 'shape'], `shape must be one of ${DIALOG_SHAPES.join(', ')}`);
	if (d.shape !== undefined && !((d.variant ?? 'bubble') === 'bubble' && d.frame === true)) c.info([...path, 'shape'], 'shape only shows on a drawn bubble (variant bubble, frame true)');
	if (d.rotation !== undefined && !isNum(d.rotation)) c.error([...path, 'rotation'], 'rotation must be a number (degrees)');
	checkScale(c, d.scale, [...path, 'scale']);
	if (d.mask !== undefined && d.mask !== true && !isStr(d.mask)) c.error([...path, 'mask'], 'mask must be true or a CSS color');
	if (d.maskShape !== undefined && !['rect', 'ellipse'].includes(d.maskShape)) c.error([...path, 'maskShape'], 'maskShape must be rect or ellipse');
	if (d.pad !== undefined && !isNum(d.pad)) c.error([...path, 'pad'], 'pad must be a number');
	if (d.tail !== undefined && !((d.variant ?? 'bubble') === 'bubble' && d.frame === true)) c.info([...path, 'tail'], 'tail only shows on a drawn bubble (variant bubble, frame true)');
}

function checkScale(c: Collector, scale: unknown, path: Path) {
	if (scale === undefined) return;
	if (!isNum(scale) || (scale as number) <= 0) c.error(path, 'scale must be a positive number (1 = original size)');
	else if ((scale as number) < 0.1 || (scale as number) > 10) c.warn(path, 'scale is outside 0.1 … 10');
}

/** A catalog item placed on a slice (`SliceItem`); its tap events are checked like a group's script. */
function checkSliceItem(c: Collector, it: any, path: Path, ctx: ValidateContext) {
	if (!isObj(it)) return c.error(path, 'item entry must be an object');
	if (!isStr(it.id) || !it.id) c.error([...path, 'id'], 'id is missing (pickups are remembered by it)');
	if (!isStr(it.item) || !it.item) c.error([...path, 'item'], 'item is missing (a catalog item id)');
	else if (ctx.items && !ctx.items.includes(it.item)) c.warn([...path, 'item'], `"${it.item}" is not in the item catalog`);
	checkRect(c, it.rect, [...path, 'rect'], false);
	if (it.rotation !== undefined && !isNum(it.rotation)) c.error([...path, 'rotation'], 'rotation must be a number (degrees)');
	if (it.action !== undefined && !(SLICE_ITEM_ACTIONS as readonly string[]).includes(it.action)) {
		c.error([...path, 'action'], `action must be one of ${SLICE_ITEM_ACTIONS.join(', ')}`);
	}
	if (it.qty !== undefined && (!Number.isInteger(it.qty) || it.qty < 1)) c.error([...path, 'qty'], 'qty must be a whole number ≥ 1');
	if (it.once !== undefined && typeof it.once !== 'boolean') c.error([...path, 'once'], 'once must be true/false');
	if (it.respawn !== undefined) {
		const rule = ctx.itemRespawn && isStr(it.item) ? (ctx.itemRespawn[it.item] ?? null) : undefined;
		const problem = placementRespawnProblem(it.respawn, rule);
		if (problem) c.error([...path, 'respawn'], problem);
		if (it.action !== 'pickup') c.info([...path, 'respawn'], 'respawn only matters for a pickup');
	}
	if (it.events !== undefined) {
		if (!Array.isArray(it.events)) c.error([...path, 'events'], 'events must be an array');
		else it.events.forEach((e: any, i: number) => checkEvent(c, e, [...path, 'events', i], ctx, false));
	}
	if (it.action === 'events' && !(Array.isArray(it.events) && it.events.length)) c.warn([...path, 'events'], 'the tap runs events, but there are none');
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
	if (target.anyOf !== undefined) {
		if (!Array.isArray(target.anyOf) || !target.anyOf.length || target.anyOf.some((l: unknown) => !isStr(l) || !l)) {
			c.error([...path, 'anyOf'], 'anyOf must be a list of group labels (one of them is picked at random)');
		}
	}
}

/** "Only if" on an event: { variable, op, value? } */
function checkCondition(c: Collector, cond: any, path: Path) {
	if (!isObj(cond) || !isStr(cond.variable) || !cond.variable) return c.error([...path, 'variable'], 'the condition needs a variable');
	if (!CONDITION_OPS.includes(cond.op)) return c.error([...path, 'op'], `the condition's op must be one of ${CONDITION_OPS.join(' ')}`);
	if (cond.op !== 'set' && cond.op !== 'unset' && (cond.value === undefined || cond.value === '')) c.error([...path, 'value'], 'the condition needs a value to compare with');
}

/** a choice's action(s): set / add / jumpTo / divert, one or a list */
function checkActions(c: Collector, onSelect: any, path: Path) {
	if (onSelect === undefined) return;
	const list = Array.isArray(onSelect) ? onSelect : [onSelect];
	list.forEach((a: any, i: number) => {
		const ap = Array.isArray(onSelect) ? [...path, i] : path;
		if (!isObj(a)) return c.error(ap, 'an action must be an object');
		if (isObj(a.jumpTo)) checkTarget(c, a.jumpTo.target, [...ap, 'jumpTo', 'target']);
		else if (isObj(a.divert)) {
			if (!isStr(a.divert.to) || !a.divert.to) c.error([...ap, 'divert', 'to'], 'divert needs the name of a point in the script');
		} else if (isObj(a.set)) {
			if (!isStr(a.set.variable) || !a.set.variable) c.error([...ap, 'set', 'variable'], 'set needs a variable');
		} else if (isObj(a.add)) {
			if (!isStr(a.add.variable) || !a.add.variable) c.error([...ap, 'add', 'variable'], 'add needs a variable');
			if (!isNum(a.add.value)) c.error([...ap, 'add', 'value'], 'add needs a number');
		} else c.error(ap, 'an action is set, add, jumpTo or divert');
	});
}

/** Character / persona / look / mood of a VN character reference against the season's cast and the registry. */
function checkCharacterRef(c: Collector, ref: { name?: unknown; persona?: unknown; look?: unknown; mood?: unknown }, path: Path, ctx: ValidateContext, key: string) {
	if (!isStr(ref.name) || !ref.name) return;
	let name: string = ref.name;
	let persona = isStr(ref.persona) && ref.persona ? ref.persona : REGULAR_PERSONA;
	// a former character that is a persona now (plf → catpope · human): resolves, but the ref should be updated
	if (ctx.expressions && LEGACY_CHARACTER_IDS[name] && !ctx.expressions[name]) {
		const resolved = resolveCharacterId(name, isStr(ref.persona) ? ref.persona : undefined);
		c.info([...path, key], `"${name}" is an old id – now ${resolved.character} · ${resolved.persona}`);
		name = resolved.character;
		persona = resolved.persona;
	}
	if (ctx.cast) {
		if (!ctx.cast[name]) {
			c.warn([...path, key], ctx.expressions?.[name] ? `character "${name}" is not in this season's cast` : `unknown character "${name}"`);
			return;
		}
	} else if (ctx.characters && !ctx.characters.includes(name)) {
		c.warn([...path, key], `unknown character "${name}"`);
		return;
	}
	const personas = ctx.expressions?.[name];
	if (!personas) return;
	const p = personas[persona];
	if (!p) {
		c.warn([...path, 'persona'], `${name} has no persona "${persona}" (${Object.keys(personas).join(', ')})`);
		return;
	}
	if (ctx.cast?.[name] && !ctx.cast[name].includes(persona)) c.warn([...path, 'persona'], `${name} · ${persona} is not in this season's cast`);
	const look = isStr(ref.look) && ref.look ? ref.look : p.defaultLook;
	if (!p.looks[look]) {
		c.warn([...path, 'look'], `${name}${persona === REGULAR_PERSONA ? '' : ` · ${persona}`} has no look "${look}" (${Object.keys(p.looks).join(', ')}) – the reader shows the default look`);
		return;
	}
	if (isStr(ref.mood) && ref.mood) {
		if (ctx.moods && !ctx.moods.includes(ref.mood)) c.warn([...path, 'mood'], `mood "${ref.mood}" is not in the mood vocabulary – the reader shows neutral`);
		else if (!p.looks[look].includes(ref.mood)) c.warn([...path, 'mood'], `${name} (${look}) has no "${ref.mood}" image – falls back to neutral`);
	}
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
			if (isObj(p) && p.character) checkCharacterRef(c, { name: p.character, persona: p.persona }, pp, ctx, 'character');
			break;
		case 'addChoice':
			if (!isObj(p) || !isStr(p.label)) c.error([...pp, 'label'], 'addChoice needs params.label');
			if (isObj(p)) checkActions(c, p.onSelect, [...pp, 'onSelect']);
			break;
		case 'label':
			if (!isObj(p) || !isStr(p.name) || !p.name) c.error([...pp, 'name'], 'a point needs a name');
			else if (!/^[a-z0-9_-]+$/i.test(p.name)) c.warn([...pp, 'name'], 'a point name uses letters, digits, - and _');
			break;
		case 'divert':
			if (!isObj(p) || !isStr(p.to) || !p.to) c.error([...pp, 'to'], 'divert needs the name of a point in the script');
			break;
		case 'setVariable':
			if (!isObj(p) || !isStr(p.variable) || !p.variable) c.error([...pp, 'variable'], 'setVariable needs a variable');
			else {
				if (p.op !== undefined && p.op !== 'set' && p.op !== 'add') c.error([...pp, 'op'], 'op is set or add');
				if (p.op === 'add' && !isNum(p.value)) c.error([...pp, 'value'], 'add needs a number');
				if (p.op !== 'add' && (p.value === undefined || p.value === '')) c.error([...pp, 'value'], 'setVariable needs a value');
			}
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
					const id = isObj(ch) ? (ch.character ?? ch.name) : undefined;
					const key = isObj(ch) && ch.character !== undefined ? 'character' : 'name';
					if (!isStr(id) || !id) c.error([...pp, 'characters', i, 'character'], 'character is missing');
					else checkCharacterRef(c, e.type === 'joinScene' ? { ...ch, name: id } : { name: id, persona: ch.persona }, [...pp, 'characters', i], ctx, key);
					const view = ch?.variant ?? ch?.position?.variant;
					if (view && !CHARACTER_VARIANTS.includes(view)) {
						c.error([...pp, 'characters', i, ch?.variant !== undefined ? 'variant' : 'position'], 'the view (variant) must be avatar, full or portrait');
					}
				});
			break;
		case 'showOnomatopeia':
			// an onomatopoeia is a text box this event reveals (owner 2026-10-03)
			if (!isObj(p) || !isStr(p.dialog) || !p.dialog) c.error([...pp, 'dialog'], 'showOnomatopeia needs params.dialog (the id of the text box it reveals)');
			if (isObj(p)) {
				if (p.effect !== undefined && !values(OnomatopoeiaEffect).includes(p.effect)) c.error([...pp, 'effect'], `effect must be one of ${values(OnomatopoeiaEffect).join(', ')}`);
				if (p.duration !== undefined && (!isNum(p.duration) || p.duration > 5000)) c.warn([...pp, 'duration'], 'duration is capped at 5000 ms by the reader');
				if (p.delay !== undefined && (!isNum(p.delay) || p.delay > 5000)) c.warn([...pp, 'delay'], 'delay is capped at 5000 ms by the reader');
				if (p.mode !== undefined && !values(TriggerMode).includes(p.mode)) c.error([...pp, 'mode'], 'mode must be mono or poly');
				if (p.wait !== undefined && typeof p.wait !== 'boolean') c.error([...pp, 'wait'], 'wait must be true/false');
				if (p.stay !== undefined && typeof p.stay !== 'boolean') c.error([...pp, 'stay'], 'stay must be true/false');
			}
			break;
	}
	if (e.if !== undefined) checkCondition(c, e.if, [...path, 'if']);
	if (sliceEvent) checkTrigger(c, e.trigger, [...path, 'trigger'], false);
}

/**
 * The named points of a group's script: unique, and every divert (event, choice action, slice event) goes to one that
 * exists – slice events divert into their group's script.
 */
function checkScriptPoints(c: Collector, g: any, path: Path) {
	const events: any[] = Array.isArray(g.events) ? g.events : [];
	const names = new Map<string, number>();
	events.forEach((e: any, i: number) => {
		if (e?.type !== 'label' || !isStr(e.params?.name) || !e.params.name) return;
		if (names.has(e.params.name)) c.error([...path, 'events', i, 'params', 'name'], `the point "${e.params.name}" is there twice`);
		else names.set(e.params.name, i);
	});
	const check = (to: unknown, at: Path) => {
		if (isStr(to) && to && !names.has(to)) c.error(at, `there is no point "${to}" in this group's script`);
	};
	const scan = (list: any[], at: Path) =>
		list.forEach((e: any, i: number) => {
			if (e?.type === 'divert') check(e.params?.to, [...at, i, 'params', 'to']);
			if (e?.type === 'addChoice') {
				const os = e.params?.onSelect;
				(Array.isArray(os) ? os : os ? [os] : []).forEach((a: any, ai: number) => {
					if (isObj(a?.divert)) check(a.divert.to, [...at, i, 'params', 'onSelect', ...(Array.isArray(os) ? [ai] : []), 'divert', 'to']);
				});
			}
		});
	scan(events, [...path, 'events']);
	if (Array.isArray(g.slices)) g.slices.forEach((s: any, si: number) => Array.isArray(s?.events) && scan(s.events, [...path, 'slices', si, 'events']));
}

function checkSlice(c: Collector, s: any, path: Path, ctx: ValidateContext) {
	if (!isObj(s)) return c.error(path, 'slice must be an object');
	if (!isStr(s.image) || !s.image) c.error([...path, 'image'], 'slice.image is missing');
	// every slice carries its size (lazy loading reserves the space; the audio triggers need real slice heights)
	for (const key of ['width', 'height'] as const) {
		if (!isNum(s[key]) || s[key] <= 0) c.error([...path, key], `slice.${key} is missing (px of the image, the standard is ${DEFAULT_SLICE.width} × ${DEFAULT_SLICE.height})`);
	}
	if (isNum(s.width) && s.width > 0 && s.width !== DEFAULT_SLICE.width) c.warn([...path, 'width'], `slices are ${DEFAULT_SLICE.width} px wide (this one is ${s.width})`);
	if (s.sensitive !== undefined && typeof s.sensitive !== 'boolean') c.error([...path, 'sensitive'], 'slice.sensitive must be true or false');
	if (s.imageVariants !== undefined) {
		if (!isObj(s.imageVariants)) c.error([...path, 'imageVariants'], 'imageVariants must be { low, medium, high } (1x / 2x / 3x paths)');
		else for (const [level, file] of Object.entries(s.imageVariants)) {
			if (!['low', 'medium', 'high'].includes(level)) c.error([...path, 'imageVariants', level], 'imageVariants levels are low / medium / high (1x / 2x / 3x)');
			else if (!isStr(file) || !file) c.error([...path, 'imageVariants', level], 'an image variant is a CDN path');
		}
	}
	if (s.video !== undefined) {
		if (!isObj(s.video) || !isStr(s.video.src) || !s.video.src) c.error([...path, 'video', 'src'], 'video.src is missing (an .mp4 on the CDN)');
		else if (!/\.(mp4|webm)$/i.test(s.video.src)) c.warn([...path, 'video', 'src'], 'video.src is not a video path (.mp4, .webm)');
		if (isObj(s.video) && s.video.sources !== undefined) {
			c.error([...path, 'video', 'sources'], 'video.sources is replaced by video.variants { low, medium, high } (one file per quality level)');
		}
		if (isObj(s.video) && s.video.variants !== undefined) {
			const variants = s.video.variants;
			if (!isObj(variants)) c.error([...path, 'video', 'variants'], 'video.variants must be { low?, medium?, high? } (480p / 720p / 1080p files)');
			else
				for (const [level, file] of Object.entries(variants)) {
					const vp = [...path, 'video', 'variants', level];
					if (!isMediaQuality(level)) c.error(vp, `unknown quality level "${level}" (low, medium, high)`);
					else if (!isStr(file) || !file) c.error(vp, `video.variants.${level} is missing (an .mp4 on the CDN)`);
					else if (!/\.(mp4|webm)$/i.test(file)) c.warn(vp, `video.variants.${level} is not a video path (.mp4, .webm)`);
				}
		}
		for (const key of ['loop', 'autoplay', 'player', 'controls'] as const) {
			if (isObj(s.video) && s.video[key] !== undefined && typeof s.video[key] !== 'boolean') c.error([...path, 'video', key], `video.${key} must be true or false`);
		}
		if (isObj(s.video) && s.video.viewport !== undefined) {
			const vp = s.video.viewport;
			if (!isObj(vp)) c.error([...path, 'video', 'viewport'], 'video.viewport must be { position?, fit? }');
			else {
				if (vp.position !== undefined && !['top', 'center', 'bottom'].includes(vp.position)) c.error([...path, 'video', 'viewport', 'position'], 'viewport.position must be top, center or bottom');
				if (vp.fit !== undefined && !(VIEWPORT_FITS as readonly string[]).includes(vp.fit)) c.error([...path, 'video', 'viewport', 'fit'], 'viewport.fit must be contain (show all), width (fit width), height (fit height) or cover (fill, cropped)');
				if (Array.isArray(s.dialog) && s.dialog.length) c.warn([...path, 'dialog'], 'text boxes are placed on the reading column; on a full-viewport video they do not follow the video');
			}
		}
		if (isObj(s.video) && s.video.frame !== undefined) {
			const fr = s.video.frame;
			const fp = [...path, 'video', 'frame'];
			if (!isObj(fr)) c.error(fp, 'video.frame must be { ratio, fit?, position? }');
			else {
				if (!parseFrameRatio(fr.ratio)) c.error([...fp, 'ratio'], 'frame.ratio must be "w:h" or [w, h] (e.g. "9:16")');
				else if (!isFrameRatioAllowed(fr.ratio)) c.error([...fp, 'ratio'], 'frame.ratio must be between 4:1 and 1:4');
				else {
					const size = frameSliceSize(fr.ratio, isNum(s.width) && s.width > 0 ? s.width : DEFAULT_SLICE.width);
					if (size && isNum(s.height) && Math.abs(s.height - size.height) > 1) c.warn([...path, 'height'], `the slice height ${s.height} doesn't follow the frame ${String(fr.ratio)} (${size.width} × ${size.height})`);
				}
				if (fr.fit !== undefined && !(VIEWPORT_FITS as readonly string[]).includes(fr.fit)) c.error([...fp, 'fit'], 'frame.fit must be contain, width, height or cover');
				if (fr.position !== undefined && !['top', 'center', 'bottom'].includes(fr.position)) c.error([...fp, 'position'], 'frame.position must be top, center or bottom');
				if (s.video.viewport !== undefined) c.warn(fp, 'a full-viewport video ignores the frame (the box is the viewport)');
			}
		}
	}
	if (s.dialog !== undefined) {
		if (!Array.isArray(s.dialog)) c.error([...path, 'dialog'], 'dialog must be an array');
		else s.dialog.forEach((d: any, i: number) => checkDialog(c, d, [...path, 'dialog', i]));
	}
	if (s.items !== undefined) {
		if (!Array.isArray(s.items)) c.error([...path, 'items'], 'items must be an array');
		else {
			s.items.forEach((it: any, i: number) => {
				checkSliceItem(c, it, [...path, 'items', i], ctx);
				if (Array.isArray(it?.events)) checkDialogRefs(c, it.events, [s], [...path, 'items', i, 'events'], 'this slice');
			});
			const ids = s.items.map((it: any) => it?.id).filter(isStr);
			ids.forEach((id: string, i: number) => {
				if (ids.indexOf(id) !== i) c.error([...path, 'items'], `item id "${id}" is used twice on this slice`);
			});
		}
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
		else {
			s.events.forEach((e: any, i: number) => checkEvent(c, e, [...path, 'events', i], ctx, true));
			checkDialogRefs(c, s.events, [s], [...path, 'events'], 'this slice');
		}
	}
}

/** onomatopoeia events that reveal a text box (`params.dialog`): the box must be there (slice events: on the slice) */
function checkDialogRefs(c: Collector, events: any[], slices: any[], path: Path, where: string) {
	const ids = new Set(slices.flatMap((s) => (Array.isArray(s?.dialog) ? s.dialog.map((d: any) => d?.id) : [])).filter(isStr));
	const seen = new Map<string, number>();
	events.forEach((e: any, i: number) => {
		const ref = e?.type === 'showOnomatopeia' ? e?.params?.dialog : undefined;
		if (!isStr(ref) || !ref) return;
		if (!ids.has(ref)) c.warn([...path, i, 'params', 'dialog'], `text box "${ref}" is not on ${where} – the event shows nothing`);
		if (seen.has(ref)) c.info([...path, i, 'params', 'dialog'], `text box "${ref}" is revealed by event ${seen.get(ref)! + 1} already`);
		else seen.set(ref, i);
	});
}

/**
 * The VN's logic in a group (owner 2026-10-03: "people need to enter before they can speak"): the scene starts empty and
 * is cleared when the group ends (viewer visual-novel.svelte.ts `clear`), so per group – the group's script first, then
 * the slices' events in reading order – a speaker must have joined and not left, nobody leaves without being there (joining
 * again updates the character on stage), and choices are added before they are shown and shown once added. Warnings: the reader still plays it.
 */
function checkVnFlow(c: Collector, g: any, path: Path) {
	const flow: { e: any; path: Path }[] = [];
	if (Array.isArray(g.events)) g.events.forEach((e: any, i: number) => flow.push({ e, path: [...path, 'events', i] }));
	if (Array.isArray(g.slices))
		g.slices.forEach((s: any, si: number) =>
			(Array.isArray(s?.events) ? s.events : []).forEach((e: any, i: number) => flow.push({ e, path: [...path, 'slices', si, 'events', i] }))
		);
	if (!flow.length) return;
	const idOf = (ref: any) => {
		const raw = isObj(ref) ? (ref.character ?? ref.name) : undefined;
		return isStr(raw) && raw ? resolveCharacterId(raw, isStr(ref.persona) ? ref.persona : undefined).character : '';
	};
	const onStage = new Set<string>();
	let choices: Path[] = [];
	for (const { e, path: ep } of flow) {
		if (!isObj(e)) continue;
		const p = isObj(e.params) ? e.params : {};
		if (e.type === 'joinScene' || e.type === 'leaveScene') {
			(Array.isArray(p.characters) ? p.characters : []).forEach((ch: any, i: number) => {
				const id = idOf(ch);
				if (!id) return;
				// joining again is fine: the reader updates the character on stage (another look / view)
				if (e.type === 'joinScene') onStage.add(id);
				else {
					if (!onStage.has(id)) c.warn([...ep, 'params', 'characters', i], `${id} leaves the scene without having joined it`);
					onStage.delete(id);
				}
			});
		} else if (e.type === 'showMessage') {
			const id = idOf(p);
			if (id && !onStage.has(id)) c.warn([...ep, 'params', 'character'], `${id} speaks without being in the scene – add a joinScene before`);
		} else if ((e.type === 'divert' || e.type === 'jumpTo') && !isObj(e.if)) {
			// the script leaves here (another point / page): choices added before are never shown
			for (const cp of choices) c.warn(cp, 'addChoice is never shown – add a showChoices after it');
			choices = [];
		} else if (e.type === 'addChoice') {
			choices.push(ep);
		} else if (e.type === 'showChoices') {
			if (!choices.length) c.warn(ep, 'showChoices without an addChoice before it (nothing to choose)');
			choices = [];
		}
	}
	for (const ep of choices) c.warn(ep, 'addChoice is never shown – add a showChoices after it');
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
		else {
			g.events.forEach((e: any, i: number) => checkEvent(c, e, [...path, 'events', i], ctx, false));
			checkDialogRefs(c, g.events, Array.isArray(g.slices) ? g.slices : [], [...path, 'events'], "the group's slices");
		}
	}
	checkScriptPoints(c, g, path);
	checkVnFlow(c, g, path);
	checkGroupLinks(c, g, path);
}

/** Linked groups (chains.ts): `continues` a boolean, `end` { target, auto? } – ignored on a group that continues */
function checkGroupLinks(c: Collector, g: any, path: Path) {
	if (g.continues !== undefined && typeof g.continues !== 'boolean') c.error([...path, 'continues'], 'continues must be true or false');
	if (g.end === undefined) return;
	if (!isObj(g.end)) return c.error([...path, 'end'], 'end must be an object { target, auto? }');
	checkTarget(c, g.end.target, [...path, 'end', 'target']);
	if (g.end.auto !== undefined && typeof g.end.auto !== 'boolean') c.error([...path, 'end', 'auto'], 'end.auto must be true or false');
	if (g.continues === true) c.warn([...path, 'end'], "end is ignored: the group continues into the next group (only a chain's last group has an end)");
}

/**
 * The chapter's chains (chains.ts): the last group can't continue, a VN group (script or slice events) only ends a chain,
 * a chain that isn't the chapter's start needs a label on its first group (else nothing reaches it). `groups` = the
 * chapter's groups with each scene in place of its ref (where known).
 */
function checkChains(c: Collector, groups: any[]) {
	groups.forEach((g, i) => {
		if (!isObj(g) || g.continues !== true) return;
		if (i === groups.length - 1) c.error(['groups', i, 'continues'], "the chapter's last group can't continue (nothing follows it on the page)");
		else if (hasVisualNovel(g)) c.error(['groups', i, 'continues'], 'a group with a VN script or slice events can only be the last group of linked groups – remove continues');
		if (i > 0 && !g.label && !linksToNext(groups, i - 1)) c.warn(['groups', i, 'label'], 'linked groups start here without a label – nothing can reach them');
	});
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
	if (ch.featuring !== undefined) {
		if (!Array.isArray(ch.featuring) || ch.featuring.some((id: unknown) => !isStr(id) || !id)) c.error(['featuring'], 'featuring must be a list of character ids');
		else {
			ch.featuring.forEach((id: string, i: number) => {
				if (ch.featuring.indexOf(id) !== i) c.warn(['featuring', i], `"${id}" is featured twice`);
				else if (ctx.cast && !ctx.cast[id]) c.warn(['featuring', i], `"${id}" is featured but not in this season's cast`);
			});
		}
	}
	checkRelatedItems(ch.related_items, ctx.items, (level, path, message) => c.add(level, path, message));
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
			checkAudioVariants(c, t, ['audio', 'music', 'tracks', i]);
		});
		if (music?.defaultTrack && !ids.includes(music.defaultTrack)) c.error(['audio', 'music', 'defaultTrack'], 'defaultTrack is not one of the tracks');
		const sids = (ch.audio?.sampleLibrary ?? []).map((t: any) => t?.id);
		if (new Set(sids).size !== sids.length) c.error(['audio', 'sampleLibrary'], 'sample ids must be unique');
		(ch.audio?.sampleLibrary ?? []).forEach((t: any, i: number) => checkAudioVariants(c, t, ['audio', 'sampleLibrary', i]));
	}
	if (!Array.isArray(ch.groups) || !ch.groups.length) c.error(['groups'], 'chapter needs at least one group');
	else {
		ch.groups.forEach((g: any, i: number) => checkGroup(c, g, ['groups', i], { ...ctx, audio: ch.audio }));
		checkChains(c, ch.groups.map((g: any) => (isObj(g) && g.ref ? (ctx.resolveGroup?.(g) ?? g) : g)));
	}
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

/**
 * `Cover` (toon entry, season): square / portrait image paths + `lettering` (the UI renders the title over text-free art):
 * true / false, or a layout per format (`CoverLettering`, cover.ts). `languages` (the toon entry's) → a custom line's
 * missing translations are warnings.
 */
function checkCover(c: Collector, cover: unknown, languages?: readonly string[]) {
	if (!isObj(cover)) return c.error(['cover'], 'cover must be { square?, portrait?, lettering? }');
	for (const key of ['square', 'portrait']) checkImagePath(c, cover[key], ['cover', key]);
	const lettering = cover.lettering;
	if (lettering !== undefined && typeof lettering !== 'boolean' && !coverLetteringOn(lettering))
		return c.error(['cover', 'lettering'], 'cover.lettering must be true, false or a layout { square?: lines, portrait?: lines }');
	if (coverLetteringOn(lettering) && !cover.square && !cover.portrait) c.warn(['cover', 'lettering'], 'cover.lettering is on, but there is no cover image');
	if (!isObj(lettering)) return;
	for (const key of Object.keys(lettering)) {
		const path = ['cover', 'lettering', key];
		if (!(COVER_FORMATS as readonly string[]).includes(key)) {
			c.warn(path, `cover.lettering.${key} is unknown – the formats are ${COVER_FORMATS.join(' and ')}`);
			continue;
		}
		const lines = lettering[key];
		if (!Array.isArray(lines)) {
			c.error(path, `cover.lettering.${key} must be a list of lines`);
			continue;
		}
		if (lines.length && !cover[key]) c.warn(path, `cover.lettering.${key} has lines, but there is no ${key} image`);
		lines.forEach((line: unknown, i: number) => checkCoverLine(c, line, [...path, i], languages));
	}
}

/** one line of a cover layout: its text source + geometry (numbers in their limits) + palette colours */
function checkCoverLine(c: Collector, line: unknown, path: Path, languages?: readonly string[]) {
	if (!isObj(line)) return c.error(path, 'a cover line must be { text, x, y, size, maxWidth, … }');
	if (!(COVER_TEXT_SOURCES as readonly string[]).includes(line.text)) c.error([...path, 'text'], `text must be one of ${COVER_TEXT_SOURCES.join(', ')} (the text comes from a text field, never from the image)`);
	if (line.text === 'custom') {
		if (!isObj(line.custom) || Object.values(line.custom).some((v) => !isStr(v))) c.error([...path, 'custom'], 'custom must be the text per language { de: "…", en: "…" }');
		else {
			const missing = (languages?.length ? languages : Object.keys(line.custom)).filter((l) => !isStr(line.custom[l]) || !line.custom[l].trim());
			if (missing.length) c.warn([...path, 'custom'], `the custom text is empty in ${missing.join(', ')}`);
		}
	} else if (line.custom !== undefined) c.warn([...path, 'custom'], `custom is only used with text "custom" (this line shows the ${line.text} title)`);
	for (const key of ['x', 'y', 'size', 'maxWidth', 'rotate', 'skew'] as const) {
		const value = line[key];
		const required = key !== 'rotate' && key !== 'skew';
		if (value === undefined && !required) continue;
		const { min, max } = COVER_LIMITS[key];
		if (!isNum(value)) c.error([...path, key], `${key} must be a number`);
		else if (value < min || value > max) c.error([...path, key], `${key} must be between ${min} and ${max}`);
	}
	if (line.quotes !== undefined && typeof line.quotes !== 'boolean') c.error([...path, 'quotes'], 'quotes must be true or false');
	if (line.align !== undefined && !(COVER_ALIGNS as readonly string[]).includes(line.align)) c.error([...path, 'align'], `align must be one of ${COVER_ALIGNS.join(', ')}`);
	for (const key of ['color', 'outline'] as const)
		if (line[key] !== undefined && !(COVER_COLORS as readonly string[]).includes(line[key])) c.error([...path, key], `${key} must be a palette colour: ${COVER_COLORS.join(', ')}`);
}

/** season1.json: meta (Season type) + the sequence (the chapters' visibility) */
function checkSeason(c: Collector, s: any, locale: string, ctx?: ValidateContext) {
	if (!isObj(s)) return c.error([], 'season file must be an object');
	if (!isStr(s.title) || !s.title.trim()) c.warn(['title'], 'title is empty');
	if (s.description !== undefined && !isStr(s.description)) c.error(['description'], 'description must be a string');
	else if (!s.description?.trim()) c.warn(['description'], 'description (plot) is empty');
	if (s.number !== undefined && !Number.isInteger(s.number)) c.error(['number'], 'number must be an integer');
	if (s.release !== undefined && (!isStr(s.release) || !DATE.test(s.release))) c.error(['release'], 'release must be a date YYYY-MM-DD');
	checkLanguage(c, s.language, locale, false);
	checkStringList(c, s.authors, ['authors']);
	if (s.cover !== undefined) checkCover(c, s.cover);
	if (s.download !== undefined) {
		if (!isObj(s.download)) c.error(['download'], 'download must be { music?, complete?, image? }');
		else {
			if (s.download.music !== undefined && !isStr(s.download.music)) c.error(['download', 'music'], 'download.music must be a path');
			if (s.download.complete != null && !isStr(s.download.complete)) c.error(['download', 'complete'], 'download.complete must be a path or null');
			checkImagePath(c, s.download.image, ['download', 'image']);
		}
	}
	if (s.trailer !== undefined) {
		if (!isObj(s.trailer)) c.error(['trailer'], 'trailer must be { loop, poster, variants?, chapter? }');
		else {
			if (!isStr(s.trailer.loop) || !s.trailer.loop) c.error(['trailer', 'loop'], 'trailer.loop is missing (an .mp4 on the CDN)');
			else if (!/\.(mp4|webm)$/i.test(s.trailer.loop)) c.warn(['trailer', 'loop'], 'trailer.loop is not a video path (.mp4, .webm)');
			if (s.trailer.poster === undefined || s.trailer.poster === '') c.error(['trailer', 'poster'], 'trailer.poster is missing (an image on the CDN)');
			else checkImagePath(c, s.trailer.poster, ['trailer', 'poster']);
			if (s.trailer.variants !== undefined) {
				if (!isObj(s.trailer.variants)) c.error(['trailer', 'variants'], 'trailer.variants must be { low?, medium?, high? } video paths');
				else
					for (const [level, file] of Object.entries(s.trailer.variants)) {
						if (!isMediaQuality(level)) c.error(['trailer', 'variants', level], `unknown quality "${level}" (low, medium, high)`);
						else if (!isStr(file) || !/\.(mp4|webm)$/i.test(file)) c.error(['trailer', 'variants', level], 'a variant must be a video path (.mp4, .webm)');
					}
			}
			if (s.trailer.sources !== undefined) {
				if (!Array.isArray(s.trailer.sources)) c.error(['trailer', 'sources'], 'trailer.sources must be a list of { src, media? }');
				else
					s.trailer.sources.forEach((source: any, i: number) => {
						if (!isObj(source) || !isStr(source.src) || !source.src) c.error(['trailer', 'sources', i, 'src'], 'source.src is missing (an .mp4 on the CDN)');
						if (isObj(source) && source.media !== undefined && !isStr(source.media)) c.error(['trailer', 'sources', i, 'media'], 'source.media must be a media query string');
					});
			}
			if (s.trailer.chapter !== undefined && (!Number.isInteger(s.trailer.chapter) || s.trailer.chapter < 0)) c.error(['trailer', 'chapter'], 'trailer.chapter must be a chapter number');
			else if (Array.isArray(s.sequence) && !s.sequence.some((item: any) => item?.number === (s.trailer.chapter ?? 0)))
				c.warn(['trailer', 'chapter'], `the trailer chapter ${s.trailer.chapter ?? 0} is not in the sequence`);
		}
	}
	checkRelatedItems(s.related_items, ctx?.items, (level, path, message) => c.add(level, path, message));
	if (s.supporters !== undefined) {
		if (!Array.isArray(s.supporters)) c.error(['supporters'], 'supporters must be a list of { name, logo?, url? }');
		else
			s.supporters.forEach((sup: any, i: number) => {
				if (!isObj(sup)) return c.error(['supporters', i], 'a supporter must be { name, logo?, url? }');
				if (!isStr(sup.name) || !sup.name.trim()) c.error(['supporters', i, 'name'], 'a supporter needs a name');
				if (sup.logo !== undefined) checkImagePath(c, sup.logo, ['supporters', i, 'logo']);
				if (sup.url !== undefined && (!isStr(sup.url) || !/^https?:\/\/\S+$/.test(sup.url))) c.error(['supporters', i, 'url'], 'url must be an http(s) link');
			});
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

/**
 * toon.json (`ToonSettings`, entity `toon-settings:<toon>:<l>`, legacy since agents/NEUTRAL.md step 4 – the toon entry
 * holds these as locale maps): the per-language settings (title, description, release, status). The language-neutral ones (`TOON_NEUTRAL_FIELDS`: map, tile, announce, cover …) belong to the index entry
 * `toon:<toon>` – a copy here is a warning (the reader takes the entry's), so they don't creep back per language.
 */
function checkToon(c: Collector, t: any, locale: string) {
	if (!isObj(t)) return c.error([], 'toon file must be an object');
	if (!isStr(t.id) || !t.id) c.error(['id'], 'id must be the toon id');
	if (!isStr(t.title) || !t.title.trim()) c.warn(['title'], 'title is empty');
	if (!isStr(t.description) || !t.description.trim()) c.warn(['description'], 'description is empty');
	checkLanguage(c, t.language, locale, true);
	if (!isStr(t.release) || !DATE.test(t.release)) c.error(['release'], 'release must be a date YYYY-MM-DD');
	if (!values(Status).includes(t.status)) c.error(['status'], `status must be one of ${values(Status).join(', ')}`);
	if (t.download !== undefined) c.warn(['download'], 'download belongs to a season now (season.download) – the reader ignores it here');
	for (const field of TOON_NEUTRAL_FIELDS) {
		if (t[field] !== undefined) c.warn([field], `${field} is the same in every language: it belongs to the toon entry (toon:${t.id || '<toon>'}) – the reader ignores it here`);
	}
	if (t.cover !== undefined) c.warn(['cover'], COVER_BELONGS_TO_SEASON);
}

/** covers are per season (owner 2026-10-02): a toon is represented by its avatar */
const COVER_BELONGS_TO_SEASON = "the cover belongs to the season (season.cover, the season's cover editor in the creator) – a toon is shown with its avatar";

/**
 * The toon's index entry (`ToonIndex`, entity `toon:<toon>`): the language-neutral settings (`TOON_NEUTRAL_FIELDS`), all
 * optional. Its texts per language (`TOON_TEXT_FIELDS`) are checked by `validateNeutral('index', …)`.
 */
function checkToonIndex(c: Collector, t: any) {
	if (!isObj(t)) return c.error([], 'toon entry must be an object');
	if (!isStr(t.id) || !t.id) c.error(['id'], 'id must be the toon id');
	if (t.authors !== undefined) checkStringList(c, t.authors, ['authors']);
	if (t.chapters !== undefined && !Number.isInteger(t.chapters)) c.error(['chapters'], 'chapters must be an integer');
	if (t.thumbnail !== undefined) checkImagePath(c, t.thumbnail, ['thumbnail']);
	if (t.cover !== undefined) c.warn(['cover'], COVER_BELONGS_TO_SEASON);
	if (t.map !== undefined) {
		// the toon's start view on the world map (agents/MAP.md §2.2); the world's size and layers live in the entity `map`
		if (!isObj(t.map)) c.error(['map'], 'map must be the start view { center?: { x, y }, zoom? }');
		else {
			for (const key of ['layers', 'width', 'height'] as const)
				if (t.map[key] !== undefined) c.warn(['map', key], `map.${key} belongs to the world map (Universe → Map) – the reader ignores it here`);
			if (t.map.center !== undefined) for (const key of ['x', 'y'] as const) if (!isNum(t.map.center?.[key])) c.error(['map', 'center', key], `map.center.${key} must be a number (world units)`);
			if (t.map.zoom !== undefined && (!isNum(t.map.zoom) || t.map.zoom <= 0)) c.error(['map', 'zoom'], 'map.zoom must be a positive number');
		}
	}
	if (t.announce !== undefined && typeof t.announce !== 'boolean') c.error(['announce'], 'announce must be true or false');
	if (t.tile !== undefined) {
		if (!isObj(t.tile)) c.error(['tile'], 'tile must be { image, color? }');
		else {
			const empty = (v: unknown) => v === undefined || v === null || v === '';
			// an emptied tile (both fields cleared in the creator) shows nothing; a colour alone needs its image
			if (empty(t.tile.image) && !empty(t.tile.color)) c.error(['tile', 'image'], 'tile.image is missing (an image on the CDN)');
			else checkImagePath(c, t.tile.image, ['tile', 'image']);
			if (!empty(t.tile.color) && (!isStr(t.tile.color) || !CSS_COLOR.test(t.tile.color.trim())))
				c.error(['tile', 'color'], 'tile.color must be a CSS colour (a name like "blue" or #rgb / #rrggbb)');
		}
	}
}

/** A toon's screenplay (`ToonScript`, entity `script:<toon>:<l>`): `{ text, meta? }`, the text is kept exactly as typed */
function checkScript(c: Collector, s: any) {
	if (!isObj(s)) return c.error([], 'script must be { text, meta? }');
	if (!isStr(s.text)) c.error(['text'], 'text must be a string (the whole screenplay)');
	if (s.meta === undefined) return;
	if (!isObj(s.meta)) return c.error(['meta'], 'meta must be { title?, author?, status?, date?, basedOn?, basedOnFrom? }');
	for (const [key, value] of Object.entries(s.meta)) {
		if (!(SCRIPT_META_FIELDS as readonly string[]).includes(key)) c.warn(['meta', key], `meta.${key} is not a script field (${SCRIPT_META_FIELDS.join(', ')})`);
		else if (value !== undefined && !isStr(value)) c.error(['meta', key], `meta.${key} must be text`);
	}
}

/** A CSS colour as the registry writes them: a name (`blue`, `darkgreen`) or a hex colour */
const CSS_COLOR = /^(?:[a-z]+|#[0-9a-f]{3}|#[0-9a-f]{6})$/i;

/**
 * Suggestions from the chapter's VN usage (info, never errors): characters its scenes use that are not in `featuring`
 * (when the chapter has a featuring list) or not in the season's cast. `scene` = `chapterSceneCharacters` of the chapter.
 */
export function featuringSuggestions(
	featuring: string[] | null | undefined,
	scene: string[],
	cast: Record<string, string[]> | null | undefined,
	file: string
): Problem[] {
	const out: Problem[] = [];
	for (const id of scene) {
		if (cast && !cast[id]) out.push({ level: 'info', file, locale: null, path: ['featuring'], message: `"${id}" appears in the scenes but not in the season's cast – add them to the cast` });
		else if (Array.isArray(featuring) && !featuring.includes(id)) out.push({ level: 'info', file, locale: null, path: ['featuring'], message: `"${id}" appears in the scenes but not in "featuring"` });
	}
	if (!Array.isArray(featuring) && scene.length) out.push({ level: 'info', file, locale: null, path: ['featuring'], message: `no "featuring" yet – the scenes suggest ${scene.join(', ')}` });
	return out;
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
	else if (kind === 'season') checkSeason(c, data, locale, ctx);
	else if (kind === 'toon') checkToon(c, data, locale);
	else if (kind === 'index') checkToonIndex(c, data);
	else if (kind === 'script') checkScript(c, data);
	else if (kind === 'map') for (const p of checkWorldMap(data)) c.add(p.level, p.path, p.message);
	// a screenplay is plain text, no media paths
	if (kind !== 'script') checkMediaLocale(c, data);
	return c.problems;
}

/** Media are language-independent (owner rule 2026-10-01): a media path with a locale segment is a warning, in every entity */
function checkMediaLocale(c: Collector, data: unknown) {
	for (const [path, value, locale] of mediaPathsWithLocale(data)) c.warn(path, mediaLocaleMessage(value, locale));
}

/** Walks every slice of a chapter/group file */
export function forEachSlice(kind: string, data: any, fn: (slice: any, path: Path) => void) {
	const groups: [any, Path][] = kind === 'chapter' ? (data?.groups ?? []).map((g: any, i: number) => [g, ['groups', i]]) : [[data, []]];
	for (const [g, gp] of groups) (g?.slices ?? []).forEach((s: any, i: number) => fn(s, [...gp, 'slices', i]));
}

const LEAF_WORD: Record<TextLeafKind, string> = { title: 'title', description: 'description', release: 'release', status: 'status', dialog: 'text', message: 'text', choice: 'label' };

/**
 * The toon entry's own problems (the language-neutral settings) + its texts resolved for the source: an empty title or
 * description, no release date (warnings – a toon is shown with them). An entry without texts yet (production before its
 * conversion: the per-language toon settings hold them) has only the first.
 */
function toonEntryProblems(data: any, languages: NeutralLanguages, ctx: ValidateContext): Problem[] {
	const problems = validateFile('index', data, languages.source, ctx).map((p) => ({ ...p, locale: null }));
	const c = new Collector(ctx.file, null);
	// no texts yet (a field that isn't a locale map is reported with the locale maps' shape)
	if (!hasToonTexts(data)) return problems;
	const toon = resolveToon(data as ToonIndex, languages.source, languages);
	if (!toon.title.trim()) c.warn(['title'], 'title is empty');
	if (!String(toon.description ?? '').trim()) c.warn(['description'], 'description is empty');
	if (!toon.release) c.warn(['release'], 'no release date yet (a date per language)');
	return [...problems, ...c.problems];
}

/**
 * A language-neutral chapter / scene (`group`) / season (agents/NEUTRAL.md), or the toon entry (`index`, step 4: its
 * title / description / release / languageStatus maps): the structure once – the document as the
 * source language's readers get it (`resolve*`), problems without a locale –, the locale maps' shape (a text per
 * language, `release` a date in every locale, `sequence[].status` a status in every locale), and per toon language
 * the completeness of the texts like `validateLocales` did for per-locale documents: a missing text (warning), the same
 * as the source (info), a text box / VN text more than 130 % of the source (warning) – those carry their locale.
 * `ctx.seasonStatus` of a chapter: its status in the season's source language (null = not in the season).
 */
export function validateNeutral(kind: string, data: any, languages: NeutralLanguages, ctx: ValidateContext): Problem[] {
	const file = ctx.file;
	if (data === null || data === undefined) return [{ level: 'error', file, locale: null, path: [], message: 'no document' }];
	if (!isObj(data)) return [{ level: 'error', file, locale: null, path: [], message: 'the document must be an object' }];
	const source = languages.source;
	const all = [...new Set([source, ...(languages.languages ?? [])])];
	const resolved =
		kind === 'chapter'
			? resolveChapter(data as ChapterDoc, source, languages)
			: kind === 'season'
				? resolveSeason(data as SeasonDoc, source, languages)
				: kind === 'index'
					? null
					: resolveGroup(data as GroupDoc, source, languages);
	const problems = kind === 'index' ? toonEntryProblems(data, languages, ctx) : validateFile(kind, resolved, source, ctx).map((p) => ({ ...p, locale: null }));
	const c = new Collector(file, null);
	const at = (locale: string) => new Collector(file, locale);
	const neutralKind: NeutralKind | 'toon' = kind === 'group' ? 'scene' : kind === 'index' ? 'toon' : (kind as NeutralKind);
	const perLocale: Collector[] = [];
	forEachText(neutralKind, data, (path, value, leaf) => {
		const word = LEAF_WORD[leaf];
		if (!isObj(value)) {
			c.error(path, `${path.join('.')} must be ${leaf === 'release' ? 'a date' : leaf === 'status' ? 'a status' : 'a text'} per language { ${source}: … }`);
			return;
		}
		for (const [locale, v] of Object.entries(value)) {
			const l = at(locale);
			perLocale.push(l);
			if (leaf === 'release') {
				if (!isStr(v) || !DATE.test(v)) l.error(path, `release must be a date YYYY-MM-DD (${locale})`);
			} else if (leaf === 'status') {
				if (!values(Status).includes(v as string)) l.error(path, `status must be published, draft or coming-soon (${locale})`);
			} else if (!(isStr(v) || (leaf === 'dialog' && Array.isArray(v) && v.every(isStr)))) {
				l.error(path, `the ${locale} ${word} must be ${leaf === 'dialog' ? 'a string or string[]' : 'a string'}`);
			}
		}
		if (leaf === 'release' || leaf === 'status') return;
		const src = textOf(value[source]).trim();
		const quote = (text: string) => (leaf === 'title' || leaf === 'description' ? '' : ` for "${text.slice(0, 40)}"`);
		if (!src) {
			const other = all.find((l) => l !== source && textOf(value[l]).trim());
			if (other) {
				const l = at(source);
				perLocale.push(l);
				l.warn(path, `missing ${source} ${word}${quote(textOf(value[other]).trim())} (the source language; readers get the ${other} text)`);
			}
			return;
		}
		for (const locale of all) {
			if (locale === source) continue;
			const tr = textOf(value[locale]).trim();
			const l = at(locale);
			perLocale.push(l);
			if (!tr) l.warn(path, `missing ${locale} ${word}${quote(src)}`);
			else if (tr === src) l.info(path, `${locale} ${word} is the same as ${source} (not translated?)`);
			else if ((leaf === 'dialog' || leaf === 'message' || leaf === 'choice') && tr.length > src.length * LENGTH_WARNING_RATIO)
				l.warn(path, `${locale} ${word} is ${Math.round((tr.length / src.length) * 100)}% of ${source} (> 130%) – check the fit`);
		}
	});
	return [...problems, ...c.problems, ...perLocale.flatMap((l) => l.problems)];
}

/** Structure + translations across the locale files of one doc (legacy per-locale documents; neutral ones: validateNeutral) */
export function validateLocales(kind: string, file: string, files: Record<string, any>, primary: string): Problem[] {
	const problems: Problem[] = [];
	const add = (level: Level, locale: string | null, path: Path, message: string) =>
		problems.push({ level, file, locale, path, message });
	const source = files[primary];
	if (!source) return problems;
	for (const [locale, data] of Object.entries(files)) {
		if (locale === primary || !data) continue;
		// related items are language-neutral (the creator writes one list to every locale)
		if ((kind === 'chapter' || kind === 'season') && JSON.stringify(source.related_items ?? []) !== JSON.stringify(data.related_items ?? [])) {
			add('warning', locale, ['related_items'], `related_items differ between ${primary} and ${locale}: they are the same in every language`);
		}
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
			if (kind === 'season' && JSON.stringify(source.cover ?? null) !== JSON.stringify(data.cover ?? null)) {
				add('warning', locale, ['cover'], `the cover differs between ${primary} and ${locale}: covers are the same in every language (the title is rendered by the UI, cover.lettering)`);
			}
			if (kind === 'season') {
				const numbers = (file: any) => (file?.sequence ?? []).map((i: any) => Number(i?.number)).sort((a: number, b: number) => a - b).join(',');
				if (numbers(source) !== numbers(data)) add('warning', locale, ['sequence'], `${locale} season lists chapters ${numbers(data) || '–'}, ${primary} lists ${numbers(source) || '–'}`);
			}
		} else if (kind === 'script') {
			// a translation may be partial (it grows scene by scene); only an empty one is worth a note
			if (textOf(source.text).trim() && !textOf(data.text).trim()) add('info', locale, ['text'], `the ${locale} script is empty`);
			continue;
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
			if ((other.sensitive === true) !== (slice.sensitive === true)) add('warning', locale, [...path, 'sensitive'], `"sensitive" differs between ${primary} and ${locale}: it is the same in every language`);
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

/** a music track's / sample's `variants` (low 96k / medium 128k / high 320k): known levels, audio paths */
function checkAudioVariants(c: Collector, t: any, path: Path) {
	if (t?.variants === undefined) return;
	if (!isObj(t.variants)) {
		c.error([...path, 'variants'], 'variants must be { low?, medium?, high? } (96k / 128k / 320k files)');
		return;
	}
	for (const [level, file] of Object.entries(t.variants)) {
		const vp = [...path, 'variants', level];
		if (!isMediaQuality(level)) c.error(vp, `unknown quality level "${level}" (low, medium, high)`);
		else if (!isStr(file) || !file) c.error(vp, `variants.${level} is missing (an .mp3 on the CDN)`);
		else if (!/\.(mp3|m4a|aac|ogg|opus|wav)$/i.test(file)) c.warn(vp, `variants.${level} is not an audio path (.mp3 …)`);
	}
}
