// The VN event forms' pure parts (COMPONENTS.md step 8): the types with a form, a new event per type, the one-line
// summary, choice actions, conditions in words, the trigger words, and the cast rows' look / mood options.
import {
	FX_IMPACTS,
	REGULAR_PERSONA,
	Trigger,
	actionsOf,
	characterIndex,
	defaultLookId,
	defaultMoodVocabulary,
	findLook,
	findPersona,
	lookLabel,
	lookPortraitMoods,
	moodLabel,
	previewText,
	resolveCharacterId,
	scriptLabels,
	sortedMoods,
	type FxImpact,
	type Path
} from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { matchRevealPreset } from '$lib/editor/event-presets.ts';
import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

/** the types the editor has a form for (another type is only named, to remove) */
export const FORM_TYPES = ['showMessage', 'addChoice', 'showChoices', 'hideMessage', 'askText', 'jumpTo', 'showTextBox', 'fx', 'unlockLocation', 'setAvatarLocation', 'migrate', 'joinScene', 'leaveScene', 'label', 'divert', 'giveItem', 'takeItem', 'setVariable'];

/** the view of a joined character is its slot in the reader (DESIGN.md → "VN layout") */
export const viewSlots = () => [
	{ value: 'avatar', label: m['creator.view_slot_avatar']() },
	{ value: 'portrait', label: m['creator.view_slot_portrait']() },
	{ value: 'full', label: m['creator.view_slot_full']() }
];

/** a new event of a type: the fields its form starts with */
export function template(type: string, ctx: { firstCharacter?: string; events: any[]; pointNames: string[] }): any {
	switch (type) {
		case 'showMessage':
			// texts per language (a language-neutral chapter / scene), filled in the form
			return { type, params: { text: {}, wait: true } };
		case 'addChoice':
			return { type, params: { label: {} } };
		case 'jumpTo':
			return { type, params: { target: { type: 'chapter', value: '1' } } };
		case 'showTextBox':
			// the text box is picked (or created) in the form
			return { type, params: { effect: 'shiver' } };
		case 'fx':
			// an impact on the slice in view (agents/FX.md)
			return { type, params: { effect: 'shake' } };
		case 'joinScene':
		case 'leaveScene':
			return { type, params: { characters: [{ character: ctx.firstCharacter ?? '', persona: 'regular', ...(type === 'joinScene' ? { variant: 'avatar' } : {}) }] } };
		case 'label':
			return { type, params: { name: `point-${scriptLabels(ctx.events).length + 1}` } };
		case 'divert':
			return { type, params: { to: ctx.pointNames[0] ?? '' } };
		case 'setVariable':
			return { type, params: { variable: '', value: '', op: 'set' } };
		case 'unlockLocation':
		case 'setAvatarLocation':
			// the place is picked from the world map in the form (agents/MAP.md §8.5a)
			return { type, params: { location: '' } };
		case 'migrate':
			// the region is picked from the world map in the form; no `who` = the reader (agents/MAP.md §8.10)
			return { type, params: { region: '' } };
		case 'giveItem':
		case 'takeItem':
			// the item is picked from the catalog in the form (BACKLOG 10.38)
			return { type, params: { item: '' } };
		case 'askText':
			// the question per language; the answer goes to a story variable (or the reader's name, the scope select)
			return { type, params: { text: {}, variable: '' } };
		default:
			return { type };
	}
}

// ── choice actions (a list, CUTOVER 10.27) ──
export const ACTION_KINDS = ['set', 'add', 'give', 'take', 'jumpTo', 'divert'] as const;
export const actionKind = (a: any) => (a?.set ? 'set' : a?.add ? 'add' : a?.give ? 'give' : a?.take ? 'take' : a?.jumpTo ? 'jumpTo' : a?.divert ? 'divert' : 'other');
export const newAction = (kind: string, pointNames: string[]): any =>
	kind === 'set'
		? { set: { variable: '', value: '' } }
		: kind === 'add'
			? { add: { variable: '', value: 1 } }
			: kind === 'give'
				? { give: { item: '' } }
				: kind === 'take'
					? { take: { item: '' } }
			: kind === 'jumpTo'
				? { jumpTo: { target: { type: 'chapter', value: '1' } } }
				: { divert: { to: pointNames[0] ?? '' } };
/** the path of action `ai` of the event at `eventPath` (a single object is stored as it is, a list by index) */
export const actionPath = (event: any, eventPath: Path, ai: number): Path => [...eventPath, 'params', 'onSelect', ...(Array.isArray(event?.params?.onSelect) ? [ai] : [])];

// ── "Only if" ──
const OP_SIGNS: Record<string, string> = { '==': '=', '!=': '≠', '<': '<', '<=': '≤', '>': '>', '>=': '≥' };
export const opWord = (op: string) => (op === 'set' ? m['creator.vn_op_set']() : op === 'unset' ? m['creator.vn_op_unset']() : (OP_SIGNS[op] ?? op));
export const conditionWords = (c: any) =>
	typeof c?.expr === 'string' ? c.expr.trim() : c?.variable ? `${c.variable} ${opWord(c.op)}${c.op === 'set' || c.op === 'unset' ? '' : ` ${c.value ?? ''}`}` : '';

/**
 * The simple test as an expression (ink logic) – what the "Expression" switch starts with: `coins >= 3`, `name == "Mia"`;
 * set / unset = has a value at all (`name != ""` / `name == ""` – a missing value equals only a missing one, so `not name`).
 */
export function conditionAsExpression(c: any): string {
	if (!c?.variable) return '';
	if (c.op === 'set') return c.variable;
	if (c.op === 'unset') return `not ${c.variable}`;
	const v = c.value;
	const literal = typeof v === 'number' || (typeof v === 'string' && v.trim() !== '' && Number.isFinite(Number(v))) ? String(v).trim() : JSON.stringify(String(v ?? ''));
	return `${c.variable} ${c.op} ${literal}`;
}

/** the trigger in words (owner 2026-10-03) */
export const triggerOptions = () => [
	{ value: Trigger.CLICK, label: m['creator.trigger_click']() },
	{ value: Trigger.HOVER, label: m['creator.trigger_hover']() },
	{ value: Trigger.INTERSECT, label: m['creator.trigger_intersect']() }
];
export const triggerWord = (event: any) => triggerOptions().find((o) => o.value === event?.trigger?.type)?.label ?? '';

export function jumpWords(t: any): string {
	if (!t) return '';
	// a chapter of another toon: "toptoy · chapter 1 · mama-hypercity"
	const where = `${t.type === 'chapter' && typeof t.toon === 'string' && t.toon ? `${t.toon} · ` : ''}${t.type ?? ''} ${t.value ?? ''}`;
	if (t.anyOf?.length) return `${where} · ${m['creator.vn_random_one']({ list: t.anyOf.join(' / ') })}`;
	return `${where}${t.label ? ` · ${t.label}` : ''}`;
}

/**
 * the line's words: what the event does, in the first language of `chain` that has the text (else any); `boxes` = the
 * text boxes a showTextBox event may reveal (it names its box by the label, never the id)
 */
export function summary(event: any, chain: readonly string[] = [], boxes: { id: string; label: string }[] = []): string {
	const p = event?.params ?? {};
	const cut = (s: unknown, n = 48) => {
		const t = previewText(s, chain).replace(/\s+/g, ' ').trim();
		return t.length > n ? `${t.slice(0, n)}…` : t;
	};
	switch (event?.type) {
		case 'showMessage':
			// the speaker is the small avatar in front of the line; the words in quotes
			return cut(p.text) ? `“${cut(p.text)}”` : '';
		case 'addChoice': {
			const acts = actionsOf(p.onSelect).map((a: any) =>
				a.set ? `${a.set.variable} = ${a.set.value}` : a.add ? `${a.add.variable} + ${a.add.value}` : a.give ? giveWords(a.give) : a.take ? takeWords(a.take) : a.divert ? `↪ ${a.divert.to}` : a.jumpTo ? `→ ${jumpWords(a.jumpTo.target)}` : ''
			);
			return `“${cut(p.label, 32)}”${acts.length ? ` ${acts.join(', ')}` : ''}`;
		}
		case 'jumpTo':
			return `→ ${jumpWords(p.target)}`;
		case 'label':
			return `◆ ${p.name ?? ''}`;
		case 'divert':
			return `↪ ${p.to ?? ''}`;
		case 'setVariable':
			return `${p.variable ?? ''} ${p.op === 'add' ? '+' : '='} ${p.value ?? ''}`;
		case 'giveItem':
			return giveWords(p);
		case 'unlockLocation':
			return `${p.location ?? ''}${p.silent ? ' · silent' : ''}`;
		case 'setAvatarLocation':
			return `reader → ${p.location ?? ''}`;
		case 'migrate':
			return `${p.who ?? 'reader'} → ${p.region ?? ''}${p.silent ? ' · silent' : ''}`;
		case 'takeItem':
			return takeWords(p);
		case 'askText':
			return `${cut(p.text) ? `“${cut(p.text, 32)}” ` : ''}→ ${p.scope === 'reader' ? 'reader.name' : (p.variable ?? '')}`;
		case 'showTextBox': {
			const preset = matchRevealPreset(p);
			const box = p.dialog ? (boxes.find((b) => b.id === p.dialog)?.label ?? m['creator.reveal_box_missing']()) : '';
			return [box, preset ? m[`creator.reveal_preset_${preset}`]() : ''].filter(Boolean).join(' · ');
		}
		case 'fx':
			return [(FX_IMPACTS as readonly string[]).includes(p.effect) ? m[`creator.fx_effect_${p.effect as FxImpact}`]() : String(p.effect ?? ''), p.target === 'screen' ? m['creator.fx_target_screen']() : ''].filter(Boolean).join(' · ');
		default:
			// joinScene / leaveScene: the avatars say who
			return '';
	}
}

/** what a giveItem / give hands out, in a line: "gives cucumber ×2" (the Flow's words) */
const giveWords = (g: any) => m['creator.flow_gives']({ list: `${g?.item || '?'}${(g?.qty ?? 1) > 1 ? ` ×${g.qty}` : ''}` });

/** what a takeItem / take takes, in a line: "takes cucumber ×5" (the Flow's words) */
const takeWords = (g: any) => m['creator.flow_takes']({ list: `${g?.item || '?'}${(g?.qty ?? 1) > 1 ? ` ×${g.qty}` : ''}` });

/** the character choices of a cast row: its persona's looks (the default first) and the moods (with an image first) */
export function castOptions(choices: CharacterChoices | undefined) {
	const vocabulary = choices?.moods ?? defaultMoodVocabulary();
	const index = characterIndex(choices?.registry);
	const locale = choices?.locale ?? 'en';
	const personaOf = (row: any) => {
		const r = resolveCharacterId(row?.character ?? row?.name ?? '', row?.persona);
		return findPersona(index.get(r.character), r.persona || REGULAR_PERSONA);
	};
	return {
		looksOf(row: any) {
			const p = personaOf(row);
			if (!p) return [];
			const main = defaultLookId(p);
			return [...p.looks].sort((a, b) => Number(b.id === main) - Number(a.id === main)).map((l) => ({ id: l.id, label: lookLabel(p, l.id, locale) }));
		},
		moodOptions(row: any) {
			const p = personaOf(row);
			const look = findLook(p, row?.look || defaultLookId(p));
			const available = lookPortraitMoods(look);
			const all = sortedMoods(vocabulary).map((x) => x.id);
			return [
				...all.filter((x) => available.includes(x)).map((x) => ({ value: x, label: moodLabel(x, locale, vocabulary) })),
				...available.filter((x) => !all.includes(x)).map((x) => ({ value: x, label: x })),
				...all.filter((x) => !available.includes(x)).map((x) => ({ value: x, label: `${moodLabel(x, locale, vocabulary)} (neutral fallback)` }))
			];
		}
	};
}
export type CastOptions = ReturnType<typeof castOptions>;
