// The Flow panel's texts on every level (owner 2026-10-06, agents/DESIGN.md "Flow panel"): what a box and an edge say.
import type { FlowEdge, FlowNode, JumpToTarget } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

export interface BoxText {
	title: string;
	meta?: string;
}

export interface FlowTextContext {
	/** the chapter whose groups are drawn (a jump there names just its group) */
	chapter?: number;
	/** a character key (`boy`, `catpope·cat`) as shown */
	who?: (key: string) => string;
	/** a chapter's status word (draft / coming-soon / published) */
	status?: (state: string) => string;
}

const clip = (s: string, n = 28) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);
const list = (items: string[] | undefined, n = 3) => (items?.length ? (items.length > n ? `${items.slice(0, n).join(', ')} +${items.length - n}` : items.join(', ')) : '');

/** how often a trigger / item tap runs, when not every time (once for good with its ✓) */
export const repeatWord = (mode: string) =>
	mode === 'once' ? `✓ ${m['creator.repeat_once']()}` : mode === 'view' ? m['creator.repeat_view']() : mode === 'interval' ? m['creator.flow_repeat_interval']() : '';

export const triggerWord = (t: string | undefined) =>
	t === 'click' ? m['creator.trigger_click']() : t === 'hover' ? m['creator.trigger_hover']() : t === 'intersect' ? m['creator.trigger_intersect']() : t === 'item' ? m['creator.flow_item_tap']() : (t ?? '');

export function targetText(t: JumpToTarget | undefined, season?: number): string {
	if (!t) return '';
	const value = t.value;
	const head =
		t.type === 'chapter'
			? t.toon
				? m['creator.flow_chapter_of_toon']({ toon: t.toon, value })
				: season !== undefined
				? m['creator.flow_chapter_in_season']({ number: value, season })
				: m['creator.flow_chapter']({ value })
			: t.type === 'season'
				? m['creator.flow_season']({ value })
				: t.type === 'toon'
					? m['creator.flow_toon']({ value })
					: t.type === 'product'
						? m['creator.flow_product']({ value })
						: m['creator.flow_xr']({ value });
	if (t.anyOf?.length) return `${head} · ${m['creator.vn_random_one']({ list: t.anyOf.join(', ') })}`;
	return t.label ? `${head} · ${t.label}` : head;
}

/** a run / step: what happens in it, one line */
function summary(n: FlowNode, ctx: FlowTextContext): string {
	const who = (keys: string[] | undefined) => list(keys?.map((k) => ctx.who?.(k) ?? k));
	const parts: string[] = [];
	if (n.messages) parts.push(m['creator.flow_messages']({ count: n.messages }) + (n.speakers?.length ? ` (${who(n.speakers)})` : ''));
	if (n.joins?.length) parts.push(m['creator.flow_joins']({ list: who(n.joins) }));
	if (n.leaves?.length) parts.push(m['creator.flow_leaves']({ list: who(n.leaves) }));
	if (n.sets?.length) parts.push(m['creator.flow_sets']({ list: list(n.sets) }));
	if (n.boxes?.length) parts.push(m['creator.flow_boxes']({ list: list(n.boxes, 2) }));
	return parts.join(' · ');
}

export function boxText(n: FlowNode, ctx: FlowTextContext = {}): BoxText {
	switch (n.kind) {
		case 'group': {
			const parts = [m['creator.flow_slices']({ count: n.count ?? 0 })];
			if (n.script) parts.push(m['creator.script']());
			if (n.deadSlices?.length) parts.push(m['creator.flow_dead_slices']({ list: n.deadSlices.map((k) => k + 1).join(', ') }));
			return { title: n.name ?? '', meta: parts.join(' · ') };
		}
		case 'label':
			return { title: `${m['creator.vn_point']()} ${n.name ?? ''}` };
		case 'choices':
			return { title: m['creator.flow_question']({ count: n.count ?? 0 }) };
		case 'divert':
			return { title: m['creator.flow_divert']({ name: n.name ?? '' }) };
		case 'jump': {
			const t = n.target;
			const here = t && t.type === 'chapter' && ctx.chapter !== undefined && Number(t.value) === ctx.chapter ? (t.anyOf?.length ? m['creator.vn_random_one']({ list: t.anyOf.join(', ') }) : (t.label ?? m['creator.vn_group_first']())) : null;
			return { title: `${m['creator.flow_jump']()} → ${here ?? targetText(t)}` };
		}
		case 'trigger': {
			const title = `${m['creator.flow_slice']({ number: (n.slice ?? 0) + 1 })} · ${triggerWord(n.trigger)}`;
			// once for good: ✓ – it runs at most once per reader (the save game checks it off)
			return { title, ...(n.repeat ? { meta: repeatWord(n.repeat) } : {}) };
		}
		case 'exit':
			return { title: n.next ? `→ ${m['creator.flow_next_chapter']()}` : `→ ${targetText(n.target, n.season)}` };
		case 'run':
		case 'step': {
			const range = n.events ? (n.events[0] === n.events[1] ? `${n.events[0] + 1}` : `${n.events[0] + 1}–${n.events[1] + 1}`) : '';
			const head = n.first ? `„${clip(n.first, 34)}“` : m['creator.flow_events']({ range });
			const meta = summary(n, ctx);
			if (n.kind === 'step' && n.event === 'askText') {
				// a question with an input line: where the answer goes, the question
				const asks = m['creator.flow_asks']({ into: n.asks || '–' });
				const question = n.first ? `„${clip(n.first, 24)}“` : '';
				if (n.condition) return { title: m['creator.vn_if_short']({ test: n.condition }), meta: [asks, question].filter(Boolean).join(' · ') };
				return { title: asks, meta: question || undefined };
			}
			if (n.kind === 'step') return { title: m['creator.vn_if_short']({ test: n.condition ?? '' }), meta: [n.first ? `„${clip(n.first, 24)}“` : '', meta].filter(Boolean).join(' · ') };
			return { title: head, meta: n.first ? meta : meta || undefined };
		}
		case 'slice':
			return { title: m['creator.flow_slice']({ number: (n.slice ?? 0) + 1 }) };
		case 'chapter': {
			const title = `${m['creator.flow_chapter_title']({ number: n.chapter ?? 0 })}${n.name ? ` · ${n.name}` : ''}`;
			const parts: string[] = [];
			if (n.state) parts.push(ctx.status?.(n.state) ?? n.state);
			parts.push(m['creator.flow_groups']({ count: n.count ?? 0 }));
			if (n.dead) parts.push(m['creator.flow_dead_groups']({ count: n.dead }));
			return { title, meta: parts.join(' · ') };
		}
		case 'place':
			return { title: m['creator.flow_places']({ list: list(n.places, 2) }) };
		case 'toon':
			return { title: n.name ?? n.toon ?? '', meta: [n.state ? (ctx.status?.(n.state) ?? n.state) : '', m['creator.flow_toon_meta']({ seasons: n.seasons?.length ?? 0, count: n.count ?? 0 })].filter(Boolean).join(' · ') };
	}
}

export const chipOf = (n: FlowNode) =>
	n.status === 'unreachable' && n.kind !== 'exit' && !n.stub
		? m['creator.reach_unreachable']()
		: n.status === 'never'
			? m['creator.reach_never']()
			: n.status === 'loop'
				? m['creator.reach_loop']()
				: n.status === 'stuck'
					? m['creator.reach_stuck']()
					: undefined;

const statusHint = (n: FlowNode) =>
	n.status === 'unreachable'
		? m['creator.reach_unreachable_hint']()
		: n.status === 'never'
			? m['creator.reach_never_hint']()
			: n.status === 'loop'
				? m['creator.reach_loop_hint']()
				: n.status === 'stuck'
					? m['creator.reach_stuck_hint']()
					: n.status === 'conditional'
					? m['creator.flow_conditional']()
					: undefined;

/** a box's tooltip after its title: its status, and that a trigger runs at most once (once for good) */
export const hintOf = (n: FlowNode) => [statusHint(n), n.kind === 'trigger' && n.repeat === 'once' ? m['creator.flow_once_hint']() : ''].filter(Boolean).join(' – ') || undefined;

export function edgeText(e: FlowEdge): string {
	const parts: string[] = [];
	if (e.label) parts.push(clip(e.label));
	if (e.count && e.count > 1 && !e.label?.includes(' · ')) parts.push(`×${e.count}`);
	if (e.kind === 'reveal') parts.push(m['creator.flow_reveal']());
	if (e.branch) parts.push(e.branch === 'else' ? m['creator.flow_else']() : e.branch === 'none' ? m['creator.flow_none']() : m['creator.flow_auto']());
	if (e.condition && e.branch !== 'else') parts.push(m['creator.vn_if_short']({ test: clip(e.condition, 32) }));
	return parts.join(' · ');
}

/** a box's size in rem (the box's CSS gets the same numbers) */
export function boxSize(n: FlowNode, d: { chip?: string; meta?: string }): { w: number; h: number } {
	const big = n.kind === 'group' || n.kind === 'chapter' || n.kind === 'toon' || n.kind === 'run' || n.kind === 'step' || (n.kind === 'trigger' && !!d.meta);
	const w = n.kind === 'run' || n.kind === 'step' ? 15 : n.kind === 'toon' || n.kind === 'chapter' ? 14 : big ? 13 : 12;
	const h = big ? 3.25 : 2 + (d.chip ? 1.25 : 0);
	return { w, h };
}
