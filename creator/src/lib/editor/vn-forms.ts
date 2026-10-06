// The VN event forms' pure parts (COMPONENTS.md step 8): the types with a form, a new event per type, the one-line
// summary, choice actions, conditions in words, the trigger words, and the cast rows' look / mood options.
import {
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
	type Path
} from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { matchRevealPreset } from '$lib/editor/event-presets.ts';
import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

/** the types the editor has a form for (another type is only named, to remove) */
export const FORM_TYPES = ['showMessage', 'addChoice', 'showChoices', 'hideMessage', 'jumpTo', 'showTextBox', 'joinScene', 'leaveScene', 'label', 'divert', 'setVariable'];

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
		case 'joinScene':
		case 'leaveScene':
			return { type, params: { characters: [{ character: ctx.firstCharacter ?? '', persona: 'regular', ...(type === 'joinScene' ? { variant: 'avatar' } : {}) }] } };
		case 'label':
			return { type, params: { name: `point-${scriptLabels(ctx.events).length + 1}` } };
		case 'divert':
			return { type, params: { to: ctx.pointNames[0] ?? '' } };
		case 'setVariable':
			return { type, params: { variable: '', value: '', op: 'set' } };
		default:
			return { type };
	}
}

// ── choice actions (a list, CUTOVER 10.27) ──
export const ACTION_KINDS = ['set', 'add', 'jumpTo', 'divert'] as const;
export const actionKind = (a: any) => (a?.set ? 'set' : a?.add ? 'add' : a?.jumpTo ? 'jumpTo' : a?.divert ? 'divert' : 'other');
export const newAction = (kind: string, pointNames: string[]): any =>
	kind === 'set'
		? { set: { variable: '', value: '' } }
		: kind === 'add'
			? { add: { variable: '', value: 1 } }
			: kind === 'jumpTo'
				? { jumpTo: { target: { type: 'chapter', value: '1' } } }
				: { divert: { to: pointNames[0] ?? '' } };
/** the path of action `ai` of the event at `eventPath` (a single object is stored as it is, a list by index) */
export const actionPath = (event: any, eventPath: Path, ai: number): Path => [...eventPath, 'params', 'onSelect', ...(Array.isArray(event?.params?.onSelect) ? [ai] : [])];

// ── "Only if" ──
const OP_SIGNS: Record<string, string> = { '==': '=', '!=': '≠', '<': '<', '<=': '≤', '>': '>', '>=': '≥' };
export const opWord = (op: string) => (op === 'set' ? m['creator.vn_op_set']() : op === 'unset' ? m['creator.vn_op_unset']() : (OP_SIGNS[op] ?? op));
export const conditionWords = (c: any) => (c?.variable ? `${c.variable} ${opWord(c.op)}${c.op === 'set' || c.op === 'unset' ? '' : ` ${c.value ?? ''}`}` : '');

/** the trigger in words (owner 2026-10-03) */
export const triggerOptions = () => [
	{ value: Trigger.CLICK, label: m['creator.trigger_click']() },
	{ value: Trigger.HOVER, label: m['creator.trigger_hover']() },
	{ value: Trigger.INTERSECT, label: m['creator.trigger_intersect']() }
];
export const triggerWord = (event: any) => triggerOptions().find((o) => o.value === event?.trigger?.type)?.label ?? '';

export function jumpWords(t: any): string {
	if (!t) return '';
	if (t.anyOf?.length) return `${t.type} ${t.value} · ${m['creator.vn_random_one']({ list: t.anyOf.join(' / ') })}`;
	return `${t.type ?? ''} ${t.value ?? ''}${t.label ? ` · ${t.label}` : ''}`;
}

/** the line's words: what the event does, in the first language of `chain` that has the text (else any) */
export function summary(event: any, chain: readonly string[] = []): string {
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
				a.set ? `${a.set.variable} = ${a.set.value}` : a.add ? `${a.add.variable} + ${a.add.value}` : a.divert ? `↪ ${a.divert.to}` : a.jumpTo ? `→ ${jumpWords(a.jumpTo.target)}` : ''
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
		case 'showTextBox': {
			const preset = matchRevealPreset(p);
			return [p.dialog, preset ? m[`creator.reveal_preset_${preset}`]() : ''].filter(Boolean).join(' · ');
		}
		default:
			// joinScene / leaveScene: the avatars say who
			return '';
	}
}

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
