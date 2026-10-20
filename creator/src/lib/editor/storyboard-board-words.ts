// What the variants board's cards, strips, placeholders and Compare's sides say (#125; the port of #62's board.svelte.js
// captionOf / titleOf / runningOf / stripTitle and compare.svelte.js sideFacts – agents/research/VARIANTS-CANVAS.md
// §3.3, §3.4): a card's caption (its name + the model), its record as the tooltip (generator, model, length, seed,
// cost in euro, date; what it came from; the prompt; the status; where the file is), a running job's caption and
// estimate, a strip's tooltip, a Compare side's facts. Words only – the rules are the tested modules'.
import { m } from '$lib/paraglide/messages.js';
import { money } from './storyboard-money.ts';
import { lengthText } from './storyboard-posters.ts';
import { placeOf, type EuroRate, type Generation, type GeneratorInfo } from './storyboard-generations.ts';
import { idOfKey, shortName, statusOf, type BoardNode } from './storyboard-board-layout.ts';
import { roleWord } from './storyboard-compare.ts';

export interface WordsAt {
	shot: string;
	generators: readonly GeneratorInfo[];
	rate: EuroRate | null;
	local: ReadonlySet<string>;
	/** the chapter's generations (a parent's name) */
	all: readonly Generation[];
}

/** a model id → its label from the generators' list (the id while that isn't loaded) */
export const modelLabel = (generators: readonly GeneratorInfo[], id: string | null | undefined) => (id ? (generators.flatMap((g) => g.models).find((x) => x.id === id)?.label ?? id) : '');
const msOf = (g: Pick<Generation, 'duration'>) => (g.duration ? g.duration * 1000 : null);
const when = (d: string | null | undefined) => (d ? String(d).slice(0, 16).replace('T', ' ') : '');
/** a parent id → its name ("g3"), else "#id" */
const nameOfId = (all: readonly Generation[], id: number) => {
	const g = all.find((x) => x.id === id);
	return g ? shortName(g) : `#${id}`;
};
/** a node key as words: the render, a take, a generation's name */
export function keyWords(key: string | null, all: readonly Generation[]): string {
	if (!key) return '';
	if (key.startsWith('render:')) return m['storyboard.board_render']();
	if (key.startsWith('take:')) return m['storyboard.board_take']({ take: key.split(':')[2] ?? '' });
	const id = idOfKey(key);
	return id === null ? key : nameOfId(all, id);
}
const STATUS = { chosen: () => m['storyboard.status_chosen'](), review: () => m['storyboard.status_review'](), rejected: () => m['storyboard.status_rejected']() };

/** a card's caption: [its name, the muted line] */
export function captionOf(node: BoardNode, at: Pick<WordsAt, 'generators'>): [string, string] {
	const r = node.record as Generation | null;
	if (node.source === 'render') return [m['storyboard.board_render'](), m['storyboard.board_render_sub']()];
	if (node.source === 'take') return [m['storyboard.board_take']({ take: node.take ?? '' }), m['storyboard.board_take_sub']()];
	if (!r) return [node.key, ''];
	if (node.source === 'upload') return [shortName(r), m['storyboard.board_dropped']()];
	if (r.generator === 'frame') return [shortName(r), m['storyboard.board_frame']()];
	return [shortName(r), modelLabel(at.generators, r.model).split(/ – | \(/)[0] || r.generator];
}

/** where a generation's file is, in words */
function placeWords(g: Generation, local: ReadonlySet<string>): string {
	const p = placeOf(g, local);
	return p.at === 'local' ? m['storyboard.gen_local']() : p.at === 'work' ? m['storyboard.gen_work']() : p.at === 'elsewhere' ? m['storyboard.gen_elsewhere']({ machine: p.machine ?? m['storyboard.gen_other_machine']() }) : '';
}

/** a card's tooltip: the record – generator, model, length, seed, cost, date; parents, prompt; the status, the file */
export function titleOf(node: BoardNode, at: WordsAt): string {
	if (node.source === 'render') return m['storyboard.board_render_hint']({ shot: at.shot });
	if (node.source === 'take') return m['storyboard.board_take_hint']({ shot: at.shot, take: node.take ?? '' });
	const r = node.record as Generation | null;
	if (!r) return node.key;
	const cost = r.cost ? money(r.cost, r.currency, at.rate) : '';
	const head = [`${at.shot} · ${shortName(r)}`, r.generator === 'upload' ? m['storyboard.board_dropped']() : r.generator, modelLabel(at.generators, r.model), r.kind, lengthText(msOf(r)), r.seed != null ? `seed ${r.seed}` : '', cost, when(r.created_at)].filter(Boolean).join(' · ');
	const status = statusOf(r);
	return [
		head,
		r.parents?.length ? m['storyboard.board_from']({ from: r.parents.map((p) => nameOfId(at.all, p)).join(' + ') }) : '',
		r.prompt ? `${r.prompt.length > 200 ? `${r.prompt.slice(0, 200)} …` : r.prompt}` : '',
		status ? `${STATUS[status]()}${r.marked_at && status !== 'chosen' ? ` ${when(r.marked_at)}` : ''}` : '',
		[r.file, placeWords(r, at.local)].filter(Boolean).join(' · ')
	].filter(Boolean).join('\n');
}

/** a running job's placeholder: its caption, its estimate (euro) and its tooltip */
export function runningOf(node: BoardNode, at: WordsAt): { caption: [string, string]; cost: string; title: string } {
	const j = node.job as Generation;
	const model = modelLabel(at.generators, j.model).split(/ – | \(/)[0] || j.generator;
	const state = j.status === 'queued' ? m['storyboard.gen_status_queued']() : m['storyboard.gen_status_running']();
	const cost = j.estimate ? money(j.estimate, j.currency, at.rate, true) : '';
	const pct = j.progress != null ? ` ${Math.round(j.progress * 100)} %` : '';
	return { caption: [model, state], cost, title: [`${at.shot} · ${model} · ${state}${pct}${cost ? ` · ${cost}` : ''}`, m['storyboard.board_running_hint']({ from: keyWords(node.parent, at.all) })].join('\n') };
}

/** a strip's tooltip: its cards, when they were rejected, what a click does */
export function stripTitle(strip: BoardNode, at: WordsAt): string {
	const names = (strip.keys ?? [strip.key]).map((k) => keyWords(k, at.all)).join(', ');
	const r = strip.record as Generation | null;
	return m['storyboard.board_strip_hint']({ names, when: when(r?.marked_at) || '–' });
}

/** what a Compare side says under its picture: model · length · seed · cost, what it came from, the prompt */
export function sideFacts(node: BoardNode | null, at: WordsAt, ms: number | null): { parts: { text: string; seed?: boolean }[]; line: string; from: string; prompt: string } {
	const of = (parts: { text: string; seed?: boolean }[], from = '', prompt = '') => {
		const list = parts.filter((p) => p.text);
		return { parts: list, line: list.map((p) => p.text).join(' · '), from, prompt };
	};
	if (!node) return of([]);
	if (node.source === 'render') return of([{ text: m['storyboard.board_render']() }], m['storyboard.board_render_sub']());
	if (node.source === 'take') return of([{ text: m['storyboard.board_take']({ take: node.take ?? '' }) }]);
	const r = node.record as Generation | null;
	if (!r) return of([]);
	const cost = r.cost ? money(r.cost, r.currency, at.rate) : '';
	const model = r.generator === 'upload' ? m['storyboard.board_dropped']() : modelLabel(at.generators, r.model).split(/ – | \(/)[0] || r.generator;
	const role = roleWord(r.inputs?.[0], r.mode);
	const roleText = role === 'start' ? m['storyboard.compare_role_start']() : role === 'still' ? m['storyboard.compare_role_still']() : role === 'video' ? m['storyboard.compare_role_video']() : '';
	const from = r.parents?.length ? `${m['storyboard.board_from']({ from: r.parents.map((p) => nameOfId(at.all, p)).join(' + ') })}${roleText ? ` (${roleText})` : ''}` : '';
	return of([{ text: model }, { text: lengthText(ms ?? msOf(r)) }, { text: r.seed != null ? `seed ${r.seed}` : '', seed: true }, { text: cost }], from, r.prompt ?? '');
}
