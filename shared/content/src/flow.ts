// The story flow as a graph (owner 2026-10-06, the creator's read-only "Flow" panel), on every level (agents/DESIGN.md
// "Flow panel"):
//   - chapter (`chapterFlow`): groups in reading order (linked groups, the follow-up bar, an authored end), the key points
//     of each group's script (named points, questions with their choices, diverts, jumps), slice triggers / item taps that
//     branch, and the places outside the chapter a jump leads to;
//   - group (`groupFlow`): one group closer up – every script event, plain ones (messages, joins / leaves, variables, text
//     boxes) collapsed into runs between the key points, its triggers, where the reader comes from and goes on to;
//   - slice (`sliceFlow`): one slice – its triggers, item taps, the script events that reveal its text boxes, their targets;
//   - season / toon (`toonFlow`): one box per chapter (status, reach in the toon, dead groups), the season order, jumps
//     and ends between chapters (the toon adds map places and the other seasons); `toonReach` = what the season order or
//     any jump in the toon reaches;
//   - SCUTOONS (`universeFlow`): one box per toon, the jumps between toons (to a toon's page or a chapter + label of
//     another toon – `target.toon`) and to products / XR (from the server's flow index; `incomingJumps` = what other
//     toons jump into one toon: entries of its reach);
//   - dead things (`deadThings`): variables set but never read, read but never set, points and labels nothing jumps to;
//   - pictures (owner 2026-10-06, "Images in the Flow"): every box names its small picture as data (`image`, `badge`,
//     `faces`) – a slice's low variant, a chapter's thumbnail, a toon's avatar, a catalog item, the characters' faces;
//     the creator resolves them to URLs (and loads them lazily).
// Statuses come from `chapterReach` (reach.ts) – the single source of truth, so the graph agrees with the chips. Pure: no
// layout, no texts of the UI.
import { groupEnd, linksToNext, type ChainGroupLike } from './chains.ts';
import type { JumpToTarget } from './content.ts';
import { analyseExpression, analyseText } from './expr/analyse.ts';
import { askVariable, chapterReach, factKey, isConditional, runRepeat, triggerKey, type ChapterReach, type Reach, type ToonFacts } from './reach.ts';
import { isStoryPath, writtenVariable } from './variables.ts';
import type { RepeatMode } from './recurrence.ts';
import { actionsOf, intoOtherToon } from './vn-script.ts';
import { REGULAR_PERSONA, resolveCharacterId } from './characters.ts';
import { pickQualityVariant, type QualityVariants } from './media-quality.ts';

/**
 * a node's state: the reach of its item, or a condition that can never be true / a loop with no way out at it / a
 * question where the reader may get stuck (stuck.ts: every choice conditional – the save refuses it)
 */
export type FlowStatus = Reach | 'never' | 'loop' | 'stuck';

export type FlowNodeKind =
	/** a group of the chapter (a scene in place of its ref) */
	| 'group'
	/** a named point of a script (`label` event) */
	| 'label'
	/** a question: the choices before a `showChoices` */
	| 'choices'
	/** a `divert` event (to a named point of the same script) */
	| 'divert'
	/** a `jumpTo` event */
	| 'jump'
	/** a slice's trigger run (click / hover / scroll) or a placed item's tap that branches (a divert, a jump, a question) */
	| 'trigger'
	/** a place outside the chapter (`target`), or the next chapter (`next`) */
	| 'exit'
	/** group / slice level: plain events in a row (messages, joins / leaves, variables, text boxes) */
	| 'run'
	/** group / slice level: one plain event that runs only if its test holds */
	| 'step'
	/** slice level: the slice */
	| 'slice'
	/** season / toon level: a chapter */
	| 'chapter'
	/** toon level: the map places that lead into a chapter */
	| 'place'
	/** SCUTOONS level: a toon */
	| 'toon';

/**
 * A box's picture as data (the creator resolves it to a small image URL – never a big one): paths relative to the toon's
 * folder like the documents store them (`s1/c2/…`, `../catpope/…`); `toon` none = the flow's own toon.
 */
export type FlowImage =
	/** a slice's image, already its low variant (1x) */
	| { kind: 'slice'; path: string; toon?: string; group?: number; slice: number }
	/** a chapter's thumbnail (the toon / season level reads it from the chapter's document) */
	| { kind: 'thumbnail'; path: string; toon?: string; chapter: number }
	/**
	 * a chapter elsewhere (another chapter / toon, the next chapter): its thumbnail, or with `label` the first slice of
	 * the group that label names (where the jump lands) – the creator looks both up (the server's flow index)
	 */
	| { kind: 'chapter'; chapter: number; toon?: string; label?: string }
	/** a toon's avatar (its entry's `avatar`) */
	| { kind: 'toon'; toon: string }
	/** a catalog item's image (an item tap) */
	| { kind: 'item'; item: string };

/** a character's face as the script names it: the registry id (legacy ids resolved) and the persona / look / mood when given */
export interface FlowFace {
	character: string;
	/** none = the regular persona */
	persona?: string;
	look?: string;
	mood?: string;
}

/** how a group's slices read: one under the other, side by side (swiper), or a full-width strip */
export type FlowLayout = 'vertical' | 'horizontal' | 'strip';

export interface FlowNode {
	id: string;
	kind: FlowNodeKind;
	status: FlowStatus;
	/** the group the node belongs to (exits: none) */
	group?: number;
	/** where it lives in the chapter (groups.g, …events.i, …slices.s.events.j, …slices.s.items.x.events.j): what to open */
	path?: (string | number)[];
	/** group: its label (or the caller's `name`); label / divert: the point's name; chapter / toon: its title */
	name?: string;
	/** group: slice count; choices: the number of choices; chapter: its groups; toon: its chapters */
	count?: number;
	/** group: slices that can't be reached (indices) while the group itself can */
	deadSlices?: number[];
	/** group: it has a script */
	script?: boolean;
	/** trigger: `intersect` / `click` / `hover` / `item` */
	trigger?: string;
	/** trigger: how often it runs when not every time (`once` = once for good: at most once per reader) */
	repeat?: Exclude<RepeatMode, 'every'>;
	/** trigger / slice: the slice index */
	slice?: number;
	/** jump / exit: where to */
	target?: JumpToTarget;
	/** exit: the next chapter (the follow-up bar after the last chain or before an unlabelled group) */
	next?: boolean;
	/** group / slice level: drawn for context only (another group, a point of the group's script) */
	stub?: boolean;
	/** run / step: the events it stands for (first, last index in their list) */
	events?: [number, number];
	/** run / step: messages shown */
	messages?: number;
	/** run / step: who speaks / joins / leaves (character id, `id·persona` for a persona other than regular) */
	speakers?: string[];
	joins?: string[];
	leaves?: string[];
	/** run / step: variables set */
	sets?: string[];
	/** run / step: catalog items given (`cucumber`, `cucumber ×2` – BACKLOG 10.38) */
	gives?: string[];
	/** run / step: catalog items taken (`cucumber ×5`) */
	takes?: string[];
	/** run / step: text boxes revealed */
	boxes?: string[];
	/** run / step: the first message (as `text` shows it) */
	first?: string;
	/** step: its event type and the test it runs under */
	event?: string;
	condition?: string;
	/** step (askText): where the answer goes – the story variable, or `reader.name` */
	asks?: string;
	/** chapter: its number; chapter / exit: the season it belongs to */
	chapter?: number;
	season?: number;
	/** chapter: its status in the season (draft / coming-soon / published); toon: the toon's status */
	state?: string;
	/** chapter: groups nothing reaches */
	dead?: number;
	/** place: the places' names; toon: its id */
	places?: string[];
	toon?: string;
	/** toon: its seasons' numbers */
	seasons?: number[];
	/** its picture: a group's first slice, a slice, a jump's / exit's landing place, a chapter's thumbnail, a toon's avatar, an item */
	image?: FlowImage;
	/** a small second picture: the target toon's avatar when a jump leads into another toon */
	badge?: FlowImage;
	/**
	 * the faces to show, in order of appearance: a group – everyone acting in its script (joins, speakers, askText); a
	 * run / step – its speakers; a question – who asks it (the last speaker before it)
	 */
	faces?: FlowFace[];
	/** group: how its slices read */
	layout?: FlowLayout;
	/** group: an entry point – a jump, an end, a random list or a map place from outside leads to its label (`?label=`) */
	entry?: boolean;
	/** trigger (an item tap): the catalog item */
	item?: string;
}

export type FlowEdgeKind =
	/** a linked group flows straight on (same page) */
	| 'continues'
	/** the follow-up bar leads to the next (labelled) group */
	| 'follow'
	/** on through the script (or past a point) */
	| 'then'
	/** a choice of a question */
	| 'choice'
	| 'divert'
	| 'jump'
	/** one of a random list (`anyOf`) */
	| 'random'
	/** the group's authored end (`Group.end`) */
	| 'end'
	/** a slice trigger / item tap of the group */
	| 'trigger'
	/** slice level: a script event reveals one of the slice's text boxes */
	| 'reveal'
	/** season / toon level: the next chapter of the season (the follow-up bar) */
	| 'order'
	/** toon level: a map place leads into a chapter */
	| 'place';

export interface FlowEdge {
	id: string;
	from: string;
	to: string;
	kind: FlowEdgeKind;
	/** a choice's text */
	label?: string;
	/** on after a test that failed (`else`), a question whose choices may all be hidden (`none`), an end that goes at once (`auto`) */
	branch?: 'else' | 'none' | 'auto';
	/** the test on the way ("coins >= 3") */
	condition?: string;
	/** the test can never pass (its variable is set nowhere in the toon) */
	never?: boolean;
	/** the status of the node it leaves from (an edge out of something unreachable is dead too) */
	status: FlowStatus;
	/** season / toon / SCUTOONS: how many ways it stands for (more than one: merged) */
	count?: number;
}

export interface ChapterFlow {
	nodes: FlowNode[];
	edges: FlowEdge[];
	reach: ChapterReach;
}

export interface FlowOptions {
	/** the chapter's number: a jump to this chapter leads to its groups instead of an exit */
	chapter?: number;
	/** the chapter's toon: a chapter target naming another toon (`target.toon`) is an exit; none = any toon-named one is */
	toon?: string;
	/** the toon's facts (`toonFacts`), passed to `chapterReach` */
	facts?: ToonFacts;
	/** the reach the caller computed already (same groups and facts) – else computed here */
	reach?: ChapterReach;
	/** a group's name (default its label, else "#n") */
	name?: (group: any, index: number) => string;
	/** a text (a choice's label: a string or a locale map) as shown – default the string or the map's first text */
	text?: (value: unknown) => string;
	/** the chapter the follow-up bar leads to after the last group (the season's next one): the next-chapter exit's picture */
	nextChapter?: number;
}

/** a condition as one line: "coins >= 3", "met set", "name == Ana"; an expression as written ("coins >= 3 && met") */
export function conditionText(c: unknown): string {
	const x = c as { variable?: unknown; op?: unknown; value?: unknown; expr?: unknown } | null | undefined;
	if (x && typeof x.expr === 'string') return x.expr.trim();
	if (!x || typeof x.variable !== 'string' || !x.variable) return '';
	if (x.op === 'set' || x.op === 'unset') return `${x.variable} ${x.op}`;
	return `${x.variable} ${typeof x.op === 'string' ? x.op : '=='} ${x.value ?? ''}`.trim();
}

const firstText = (v: unknown): string => {
	if (typeof v === 'string') return v;
	if (v && typeof v === 'object') for (const t of Object.values(v)) if (typeof t === 'string' && t.trim()) return t;
	return '';
};

const conditional = isConditional;
const RANK: Record<FlowStatus, number> = { reachable: 3, conditional: 2, loop: 1, never: 1, stuck: 1, unreachable: 0 };
const KEY_EVENTS = new Set(['label', 'addChoice', 'showChoices', 'divert', 'jumpTo']);
/** the story variable an askText's answer goes to, or the reader's name */
const asksInto = (e: any): string => askVariable(e) ?? (e?.params?.scope === 'reader' ? 'reader.name' : '');
/** a character ref as one key: `boy`, `catpope·cat` */
const who = (c: any): string => {
	const id = String(c?.character ?? c?.name ?? '');
	return c?.persona && c.persona !== 'regular' ? `${id}·${c.persona}` : id;
};

/** a character ref (a join's, a message's) as a face: legacy ids resolved, persona / look / mood when given */
function faceOf(c: any): FlowFace | null {
	const id = c?.character ?? c?.name;
	if (typeof id !== 'string' || !id) return null;
	const r = resolveCharacterId(id, typeof c?.persona === 'string' ? c.persona : undefined);
	return {
		character: r.character,
		...(r.persona !== REGULAR_PERSONA ? { persona: r.persona } : {}),
		...(typeof c?.look === 'string' && c.look ? { look: c.look } : {}),
		...(typeof c?.mood === 'string' && c.mood ? { mood: c.mood } : {})
	};
}
/** adds a face once per character + persona (its first appearance keeps its look / mood) */
function addFace(list: FlowFace[], face: FlowFace | null) {
	if (face && !list.some((f) => f.character === face.character && f.persona === face.persona)) list.push(face);
}
/** everyone acting in a script, in order of appearance: who joins, who speaks, who asks */
export function scriptFaces(events: readonly any[] | null | undefined): FlowFace[] {
	const out: FlowFace[] = [];
	for (const e of Array.isArray(events) ? events : []) {
		if (e?.type === 'joinScene') for (const c of Array.isArray(e.params?.characters) ? e.params.characters : []) addFace(out, faceOf(c));
		else if ((e?.type === 'showMessage' || e?.type === 'askText') && e.params?.character) addFace(out, faceOf(e.params));
	}
	return out;
}
/** a group's layout: vertical (default), horizontal (swiper) or a full-width strip */
export function groupLayout(g: any): FlowLayout {
	if (g?.renderer?.type !== 'horizontal') return 'vertical';
	return g.renderer.params?.fullWidth ? 'strip' : 'horizontal';
}
/** a slice's picture: its low variant (1x); none without an image */
export function sliceImage(slice: any, k: number, extra: { toon?: string; group?: number } = {}): FlowImage | undefined {
	const src = typeof slice?.image === 'string' ? slice.image.trim() : '';
	if (!src) return undefined;
	const variants = slice?.imageVariants && typeof slice.imageVariants === 'object' ? (slice.imageVariants as QualityVariants) : undefined;
	return { kind: 'slice', path: pickQualityVariant({ src, variants }, 'low'), slice: k, ...(extra.toon ? { toon: extra.toon } : {}), ...(extra.group !== undefined ? { group: extra.group } : {}) };
}
/**
 * Where a jump / an end outside the chapter lands, as pictures: a chapter (+ label) – its thumbnail or the label's first
 * slice, with the toon's avatar as the badge when it is another toon's; a toon page – its avatar. `own` = the flow's toon.
 */
export function targetPictures(t: Partial<JumpToTarget> | null | undefined, own?: string): Pick<FlowNode, 'image' | 'badge'> {
	if (!t || typeof t !== 'object') return {};
	if (t.type === 'toon' && typeof t.value === 'string' && t.value) return { image: { kind: 'toon', toon: t.value } };
	if (t.type !== 'chapter' || t.value === undefined || String(t.value) === '' || !Number.isFinite(Number(t.value))) return {};
	const other = intoOtherToon(t, own) ? (t.toon as string) : undefined;
	const label = typeof t.label === 'string' && t.label ? t.label : undefined;
	return {
		image: { kind: 'chapter', chapter: Number(t.value), ...(other ? { toon: other } : {}), ...(label ? { label } : {}) },
		...(other ? { badge: { kind: 'toon', toon: other } as FlowImage } : {})
	};
}

type Path = (string | number)[];
type Pending = { from: string; kind: FlowEdgeKind; label?: string; branch?: FlowEdge['branch']; condition?: string; never?: boolean };
interface ScriptWalk {
	status: (j: number) => FlowStatus;
	id: (j: number) => string;
	path: (j: number) => Path;
	gi: number;
	pending: Pending[];
}
interface TriggerRun {
	trigger: string;
	/** an item tap: the catalog item */
	item?: string;
	repeat: RepeatMode;
	events: any[];
	idx: number[];
	base: Path;
}

/** the slice's trigger runs (one per trigger: click / hover / scroll) and its placed items' taps, in order */
function runsOf(slice: any, gi: number, k: number): TriggerRun[] {
	const runs = new Map<string, TriggerRun>();
	(Array.isArray(slice?.events) ? slice.events : []).forEach((e: any, j: number) => {
		if (!e?.trigger) return;
		const key = triggerKey(e.trigger);
		if (!runs.has(key)) runs.set(key, { trigger: String(e.trigger.type ?? ''), repeat: runRepeat(e.trigger).mode, events: [], idx: [], base: ['groups', gi, 'slices', k, 'events'] });
		runs.get(key)!.events.push(e);
		runs.get(key)!.idx.push(j);
	});
	(Array.isArray(slice?.items) ? slice.items : []).forEach((item: any, x: number) => {
		if (!Array.isArray(item?.events) || !item.events.length) return;
		runs.set(`item${x}`, { trigger: 'item', ...(typeof item.item === 'string' && item.item ? { item: item.item } : {}), repeat: runRepeat(item).mode, events: item.events, idx: item.events.map((_: unknown, j: number) => j), base: ['groups', gi, 'slices', k, 'items', x, 'events'] });
	});
	return [...runs.values()];
}

/** builds one level's graph: nodes, edges and the walks every level shares */
class Builder {
	readonly nodes: FlowNode[] = [];
	readonly edges: FlowEdge[] = [];
	readonly byId = new Map<string, FlowNode>();
	readonly loops: Set<string>;
	readonly never: Set<string>;
	readonly stuck: Set<string>;
	readonly labelled = new Map<string, number>();
	readonly text: (value: unknown) => string;
	readonly name: (group: any, index: number) => string;

	constructor(
		readonly groups: readonly any[],
		readonly options: FlowOptions,
		readonly reach: ChapterReach
	) {
		this.text = options.text ?? firstText;
		this.name = options.name ?? ((g: any, i: number) => (typeof g?.label === 'string' && g.label ? g.label : `#${i + 1}`));
		this.loops = new Set(reach.loops.map((p) => p.join('.')));
		this.never = new Set(reach.neverTrue.map((p) => p.slice(0, -1).join('.')));
		this.stuck = new Set((reach.stuck ?? []).map((p) => p.join('.')));
		groups.forEach((g, gi) => typeof g?.label === 'string' && g.label && !this.labelled.has(g.label) && this.labelled.set(g.label, gi));
	}

	add(n: FlowNode): string {
		if (!this.byId.has(n.id)) {
			this.byId.set(n.id, n);
			this.nodes.push(n);
		}
		return n.id;
	}
	edge(from: string, to: string, kind: FlowEdgeKind, extra: Partial<FlowEdge> = {}) {
		this.edges.push({ id: `${from}>${to}#${this.edges.length}`, from, to, kind, status: this.byId.get(from)?.status ?? 'reachable', ...extra });
	}
	statusAt(path: Path, base: Reach): FlowStatus {
		const key = path.join('.');
		return this.stuck.has(key) ? 'stuck' : this.loops.has(key) ? 'loop' : this.never.has(key) ? 'never' : base;
	}
	/** a group's box (`stub`: drawn for context on the group / slice level) */
	group(gi: number, stub = false): string {
		const id = `g${gi}`;
		if (this.byId.has(id)) return id;
		const g = this.groups[gi];
		const slices: any[] = Array.isArray(g?.slices) ? g.slices : [];
		const status = this.reach.groups[gi] ?? 'unreachable';
		const dead = status === 'unreachable' ? [] : (this.reach.slices[gi] ?? []).flatMap((r, k) => (r === 'unreachable' ? [k] : []));
		const image = this.firstSlice(gi);
		const faces = scriptFaces(g?.events);
		const facts = this.options.facts;
		const entry = gi > 0 && !!facts && typeof g?.label === 'string' && !!g.label && facts.labels.has(g.label);
		return this.add({
			id,
			kind: 'group',
			status,
			group: gi,
			path: ['groups', gi],
			name: this.name(g, gi),
			count: slices.length,
			...(dead.length ? { deadSlices: dead } : {}),
			...(Array.isArray(g?.events) && g.events.length ? { script: true } : {}),
			...(stub ? { stub: true } : {}),
			layout: groupLayout(g),
			...(image ? { image } : {}),
			...(faces.length ? { faces } : {}),
			...(entry ? { entry: true } : {})
		});
	}
	/** a group's first slice as its picture (where a jump to its label lands) */
	firstSlice(gi: number): FlowImage | undefined {
		const slices = this.groups[gi]?.slices;
		return Array.isArray(slices) ? sliceImage(slices[0], 0, { group: gi, ...(this.options.toon ? { toon: this.options.toon } : {}) }) : undefined;
	}
	/** a jump's picture: where it lands – a group of this chapter (its first slice), else the chapter / toon outside */
	jumpPictures(t: any): Pick<FlowNode, 'image' | 'badge'> {
		if (!t || typeof t !== 'object') return {};
		const here = t.type === 'chapter' && !intoOtherToon(t, this.options.toon) && this.options.chapter !== undefined && Number(t.value) === this.options.chapter;
		if (!here) return targetPictures(t, this.options.toon);
		if (Array.isArray(t.anyOf) && t.anyOf.length) return {};
		const gi = typeof t.label === 'string' && t.label ? this.labelled.get(t.label) : 0;
		const image = gi === undefined ? undefined : this.firstSlice(gi);
		return image ? { image } : {};
	}
	/** where a jump leads: groups of this chapter, else an exit node */
	targets(t: any): { id: string; random: boolean }[] {
		if (!t || typeof t !== 'object' || typeof t.type !== 'string') return [];
		const here = t.type === 'chapter' && !intoOtherToon(t, this.options.toon) && this.options.chapter !== undefined && Number(t.value) === this.options.chapter;
		const anyOf: string[] = Array.isArray(t.anyOf) ? t.anyOf.filter((l: unknown) => typeof l === 'string' && l) : [];
		if (here) {
			const labels = anyOf.length ? anyOf : typeof t.label === 'string' && t.label ? [t.label] : [];
			if (!labels.length) return this.groups.length ? [{ id: this.group(0, true), random: false }] : [];
			return labels.map((l) => {
				const gi = this.labelled.get(l);
				if (gi !== undefined) return { id: this.group(gi, true), random: anyOf.length > 0 };
				const target: JumpToTarget = { type: t.type, value: String(t.value), label: l };
				return { id: this.add({ id: `x:${t.type}:${t.value}:${l}`, kind: 'exit', status: 'unreachable', target }), random: anyOf.length > 0 };
			});
		}
		const target: JumpToTarget = { type: t.type, value: String(t.value ?? '') };
		const other = intoOtherToon(t, this.options.toon);
		if (other) target.toon = t.toon;
		if (typeof t.label === 'string' && t.label) target.label = t.label;
		if (anyOf.length) target.anyOf = anyOf;
		const id = `x:${t.type}:${other ? `${t.toon}/` : ''}${target.value}:${target.label ?? ''}:${anyOf.join('|')}`;
		return [{ id: this.add({ id, kind: 'exit', status: 'unreachable', target, ...targetPictures(target, this.options.toon) }), random: false }];
	}
	jumpEdges(from: string, t: unknown, extra: Partial<FlowEdge>) {
		for (const x of this.targets(t)) this.edge(from, x.id, x.random ? 'random' : 'jump', extra);
	}
	cond(e: any): Partial<FlowEdge> {
		return conditional(e) ? { condition: conditionText(e.if) } : {};
	}
	carry(p: Pending): Partial<FlowEdge> {
		return {
			...(p.label ? { label: p.label } : {}),
			...(p.branch ? { branch: p.branch } : {}),
			...(p.condition ? { condition: p.condition } : {}),
			...(p.never ? { never: true } : {})
		};
	}

	/**
	 * A script's nodes in order; returns what falls through its end. `detail`: plain events too (runs / steps), else only
	 * the key points (the chapter level). `w.pending` = what leads into the script's start.
	 */
	script(events: any[], w: ScriptWalk, detail: boolean): Pending[] {
		const { status, id, path, gi } = w;
		let pending = w.pending;
		const points = new Map<string, string>();
		events.forEach((e, j) => {
			if (e?.type === 'label' && typeof e.params?.name === 'string' && e.params.name && !points.has(e.params.name)) {
				points.set(e.params.name, this.add({ id: id(j), kind: 'label', status: status(j), group: gi, path: path(j), name: e.params.name }));
			}
		});
		let block: number[] = [];
		/** who asks the next question: the last speaker so far */
		let asker: FlowFace | null = null;
		let run: FlowNode | null = null;
		const enter = (to: string, extra: Partial<FlowEdge> = {}) => {
			for (const p of pending) this.edge(p.from, to, p.kind, { ...this.carry(p), ...extra });
			pending = [];
		};
		events.forEach((e, j) => {
			const dead = this.never.has(path(j).join('.'));
			const type = e?.type;
			if ((type === 'showMessage' || type === 'askText') && e.params?.character) asker = faceOf(e.params);
			if (KEY_EVENTS.has(type)) run = null;
			switch (type) {
				case 'label': {
					const at = points.get(e.params?.name);
					if (at !== id(j)) return; // a duplicate name: the first one counts
					enter(at);
					pending = [{ from: at, kind: 'then' }];
					return;
				}
				case 'addChoice':
					block.push(j);
					return;
				case 'divert':
				case 'jumpTo': {
					const at = this.add({
						id: id(j),
						kind: type === 'divert' ? 'divert' : 'jump',
						status: status(j),
						group: gi,
						path: path(j),
						...(type === 'divert' ? { name: String(e.params?.to ?? '') } : { target: e.params?.target, ...this.jumpPictures(e.params?.target) })
					});
					enter(at);
					const extra = { ...this.cond(e), ...(dead ? { never: true } : {}) };
					if (type === 'divert') {
						const to = points.get(e.params?.to);
						if (to) this.edge(at, to, 'divert', extra);
						// a divert to a point that isn't there goes on (as the reader does)
						if (!to || conditional(e)) pending = [{ from: at, kind: 'then', ...(to ? { branch: 'else' as const } : {}) }];
					} else {
						this.jumpEdges(at, e.params?.target, extra);
						if (conditional(e)) pending = [{ from: at, kind: 'then', branch: 'else' }];
					}
					return;
				}
				case 'showChoices': {
					const choices = block;
					block = [];
					const at = this.add({ id: id(j), kind: 'choices', status: status(j), group: gi, path: path(j), count: choices.length, ...(asker ? { faces: [asker] } : {}) });
					enter(at);
					const extra = { ...this.cond(e), ...(dead ? { never: true } : {}) };
					let sure = false;
					for (const c of choices) {
						const choice = events[c];
						const label = this.text(choice?.params?.label) || `#${choices.indexOf(c) + 1}`;
						const deadChoice = this.never.has(path(c).join('.'));
						if (!conditional(choice)) sure = true;
						const own: Partial<FlowEdge> = { label, ...(conditional(choice) ? { condition: conditionText(choice.if) } : extra.condition ? { condition: extra.condition } : {}), ...(deadChoice || dead ? { never: true } : {}) };
						const actions = actionsOf(choice?.params?.onSelect) as any[];
						const jump = actions.find((x) => x && 'jumpTo' in x);
						if (jump) {
							this.jumpEdges(at, jump.jumpTo?.target, own);
							continue;
						}
						const divert = actions.find((x) => x && 'divert' in x);
						const to = divert ? points.get(divert.divert?.to) : undefined;
						if (to) this.edge(at, to, 'choice', own);
						else pending.push({ from: at, kind: 'choice', ...own } as Pending);
					}
					// no choice, or only conditional ones: the question may be skipped
					if (!choices.length || !sure) pending.push({ from: at, kind: 'then', ...(choices.length ? { branch: 'none' as const } : {}) });
					if (conditional(e)) pending.push({ from: at, kind: 'then', branch: 'else' });
					return;
				}
			}
			if (!detail) return;
			// a plain event (message, join / leave, variable, text box …); a question with an input line is a step of its own
			if (conditional(e) || type === 'askText') {
				run = null;
				const condition = conditional(e) ? conditionText(e.if) : '';
				const at = this.add({ id: id(j), kind: 'step', status: status(j), group: gi, path: path(j), events: [j, j], event: String(type ?? ''), ...(condition ? { condition } : {}) });
				this.summarize(this.byId.get(at)!, e);
				// in line (the test only skips this one event – the flow goes on either way): the edge into it carries the test
				enter(at, condition ? { condition, ...(dead ? { never: true } : {}) } : {});
				pending = [{ from: at, kind: 'then' }];
				return;
			}
			if (run) {
				run.events![1] = j;
				if (RANK[status(j)] > RANK[run.status]) run.status = status(j);
				this.summarize(run, e);
				return;
			}
			const at = this.add({ id: id(j), kind: 'run', status: status(j), group: gi, path: path(j), events: [j, j] });
			run = this.byId.get(at)!;
			this.summarize(run, e);
			enter(at);
			pending = [{ from: at, kind: 'then' }];
		});
		return pending;
	}

	/** what a plain event adds to its run / step */
	summarize(n: FlowNode, e: any) {
		const push = (key: 'speakers' | 'joins' | 'leaves' | 'sets' | 'gives' | 'takes' | 'boxes', v: string) => {
			if (!v) return;
			const list = (n[key] ??= []);
			if (!list.includes(v)) list.push(v);
		};
		switch (e?.type) {
			case 'showMessage':
				n.messages = (n.messages ?? 0) + 1;
				if (e.params?.character) (push('speakers', who(e.params)), addFace((n.faces ??= []), faceOf(e.params)));
				if (n.first === undefined) {
					const t = this.text(e.params?.text);
					if (t) n.first = t;
				}
				break;
			case 'joinScene':
				for (const c of Array.isArray(e.params?.characters) ? e.params.characters : []) push('joins', who(c));
				break;
			case 'leaveScene':
				for (const c of Array.isArray(e.params?.characters) ? e.params.characters : []) push('leaves', who(c));
				break;
			case 'setVariable':
				push('sets', String(e.params?.variable ?? ''));
				break;
			case 'takeItem':
				push('takes', `${e.params?.item ?? ''}${(e.params?.qty ?? 1) > 1 ? ` ×${e.params.qty}` : ''}`);
				break;
			case 'giveItem':
				push('gives', `${e.params?.item ?? ''}${(e.params?.qty ?? 1) > 1 ? ` ×${e.params.qty}` : ''}`);
				break;
			case 'showTextBox':
				push('boxes', String(e.params?.dialog ?? ''));
				break;
			case 'askText': {
				const t = this.text(e.params?.text);
				if (t && n.first === undefined) n.first = t;
				const into = asksInto(e);
				if (into) n.asks = into;
				if (e.params?.character) (push('speakers', who(e.params)), addFace((n.faces ??= []), faceOf(e.params)));
				break;
			}
		}
	}

	/** a group's script from its box; returns what falls through its end */
	groupScript(gi: number, detail: boolean): Pending[] {
		const g = this.groups[gi];
		const events: any[] = Array.isArray(g?.events) ? g.events : [];
		const base = ['groups', gi, 'events'];
		const start: Pending[] = [{ from: `g${gi}`, kind: 'then' }];
		if (!events.length) return start;
		return this.script(
			events,
			{ status: (j) => this.statusAt([...base, j], this.reach.events[gi]?.[j] ?? 'unreachable'), id: (j) => `g${gi}e${j}`, path: (j) => [...base, j], gi, pending: start },
			detail
		);
	}

	/**
	 * Slice triggers and item taps: a node each. `detail`: every run and its plain events (group / slice level), else only
	 * the runs that branch with their key points (chapter level). `from(k)` = the box they hang off; `only` = one slice;
	 * `points`: a divert into the group's script draws that point as a stub when the script isn't drawn.
	 */
	triggers(gi: number, o: { detail: boolean; from: (k: number) => string; only?: number; points?: boolean }) {
		const g = this.groups[gi];
		const slices: any[] = Array.isArray(g?.slices) ? g.slices : [];
		const script: any[] = Array.isArray(g?.events) ? g.events : [];
		slices.forEach((s, k) => {
			if (o.only !== undefined && k !== o.only) return;
			const sliceStatus = this.reach.slices[gi]?.[k] ?? 'unreachable';
			let n = 0;
			for (const run of runsOf(s, gi, k)) {
				const branches = run.events.some((e) => e?.type === 'divert' || e?.type === 'jumpTo' || e?.type === 'showChoices');
				if (!branches && !o.detail) continue;
				const id = `g${gi}s${k}t${n++}`;
				this.add({ id, kind: 'trigger', status: sliceStatus, group: gi, path: [...run.base, run.idx[0]], slice: k, trigger: run.trigger, ...(run.item ? { item: run.item, image: { kind: 'item', item: run.item } as FlowImage } : {}), ...(run.repeat !== 'every' ? { repeat: run.repeat } : {}) });
				this.edge(o.from(k), id, 'trigger');
				// a lone divert goes into the group's script at its point
				const lone = run.events.length === 1 && run.events[0]?.type === 'divert' ? run.events[0] : null;
				if (lone) {
					const at = script.findIndex((e: any) => e?.type === 'label' && e.params?.name === lone.params?.to);
					const point = `g${gi}e${at}`;
					if (at >= 0 && !this.byId.has(point) && o.points) {
						this.add({ id: point, kind: 'label', status: this.statusAt(['groups', gi, 'events', at], this.reach.events[gi]?.[at] ?? 'unreachable'), group: gi, path: ['groups', gi, 'events', at], name: String(lone.params?.to ?? ''), stub: true });
					}
					if (at >= 0 && this.byId.has(point)) {
						this.edge(id, point, 'divert', { ...this.cond(lone), ...(this.never.has([...run.base, run.idx[0]].join('.')) ? { never: true } : {}) });
						continue;
					}
				}
				// its own little script: what branches in it hangs off the trigger node (its end goes nowhere: the page goes on)
				this.script(
					run.events,
					{ status: (j) => this.statusAt([...run.base, run.idx[j]], sliceStatus), id: (j) => `${id}e${j}`, path: (j) => [...run.base, run.idx[j]], gi, pending: [{ from: id, kind: 'then' }] },
					o.detail
				);
			}
		});
	}

	/** reading order after a group: linked groups, the follow-up bar, an authored end, the next chapter */
	after(gi: number, out: Pending[]) {
		if (!out.length) return;
		const g = this.groups[gi];
		const link = (to: string, kind: FlowEdgeKind, extra: Partial<FlowEdge> = {}) => {
			for (const p of out) this.edge(p.from, to, p.kind === 'choice' ? 'choice' : kind, { ...this.carry(p), ...extra });
		};
		if (linksToNext(this.groups as ChainGroupLike[], gi)) return link(this.group(gi + 1, true), 'continues');
		const end = groupEnd(g);
		if (end) {
			for (const x of this.targets(end.target)) link(x.id, x.random ? 'random' : 'end', end.auto ? { branch: 'auto' } : {});
			return;
		}
		const next = this.groups[gi + 1];
		if (next && typeof next.label === 'string' && next.label) return link(this.group(gi + 1, true), 'follow');
		const nextChapter = this.options.nextChapter;
		link(this.add({ id: 'x:next', kind: 'exit', status: 'unreachable', next: true, ...(nextChapter !== undefined ? { image: { kind: 'chapter', chapter: nextChapter } as FlowImage } : {}) }), 'follow');
	}

	/** an exit is as reachable as the best way into it */
	exits() {
		const best = new Map<string, number>();
		for (const e of this.edges) {
			const to = this.byId.get(e.to);
			if (to?.kind !== 'exit') continue;
			const r = e.never ? 0 : Math.min(RANK[e.status], e.condition ? 2 : 3);
			best.set(e.to, Math.max(best.get(e.to) ?? 0, r));
		}
		for (const n of this.nodes) if (n.kind === 'exit') n.status = (best.get(n.id) ?? 0) >= 3 ? 'reachable' : (best.get(n.id) ?? 0) >= 1 ? 'conditional' : 'unreachable';
	}

	done(): ChapterFlow {
		this.exits();
		return { nodes: this.nodes, edges: this.edges, reach: this.reach };
	}
}

/**
 * The flow of a chapter. `groups` = the chapter's groups with each scene in place of its ref (what `chapterReach` gets).
 */
export function chapterFlow(groups: readonly any[], options: FlowOptions = {}): ChapterFlow {
	const reach = options.reach ?? chapterReach(groups, options.facts);
	const b = new Builder(groups, options, reach);
	groups.forEach((_, gi) => b.group(gi));
	const through = groups.map((_, gi) => b.groupScript(gi, false));
	groups.forEach((_, gi) => b.triggers(gi, { detail: false, from: () => `g${gi}` }));
	groups.forEach((_, gi) => b.after(gi, through[gi]));
	return b.done();
}

/**
 * One group closer up: every event of its script (plain ones collapsed into runs between the key points, a plain event
 * under a test as a step), its slice triggers and item taps with their own events, the groups the reader comes from
 * (reading order, jumps – as stubs) and where it goes on to. Same statuses as `chapterFlow`.
 */
export function groupFlow(groups: readonly any[], gi: number, options: FlowOptions = {}): ChapterFlow {
	const reach = options.reach ?? chapterReach(groups, options.facts);
	const b = new Builder(groups, options, reach);
	if (!groups[gi]) return b.done();
	const head = b.group(gi);
	// where the reader comes from: the chapter's flow, edges into this group from other groups
	const whole = chapterFlow(groups, { ...options, reach });
	const wholeById = new Map(whole.nodes.map((n) => [n.id, n]));
	const seen = new Set<string>();
	for (const e of whole.edges) {
		if (e.to !== head) continue;
		const from = wholeById.get(e.from);
		if (from?.group === undefined || from.group === gi) continue;
		const key = `${from.group}|${e.kind}|${e.label ?? ''}|${e.condition ?? ''}`;
		if (seen.has(key)) continue;
		seen.add(key);
		const stub = b.group(from.group, true);
		b.edge(stub, head, e.kind, { ...(e.label ? { label: e.label } : {}), ...(e.branch ? { branch: e.branch } : {}), ...(e.condition ? { condition: e.condition } : {}), ...(e.never ? { never: true } : {}), status: e.status });
	}
	const out = b.groupScript(gi, true);
	b.triggers(gi, { detail: true, from: () => head });
	b.after(gi, out);
	return b.done();
}

/**
 * One slice closer up: the slice, its group (a stub), its trigger runs and item taps with every event, the points of the
 * group's script they divert into (stubs), the script events that reveal the slice's text boxes, and where it all leads.
 */
export function sliceFlow(groups: readonly any[], gi: number, k: number, options: FlowOptions = {}): ChapterFlow {
	const reach = options.reach ?? chapterReach(groups, options.facts);
	const b = new Builder(groups, options, reach);
	const g = groups[gi];
	const slice = Array.isArray(g?.slices) ? g.slices[k] : undefined;
	if (!slice) return b.done();
	const group = b.group(gi, true);
	const image = sliceImage(slice, k, { group: gi, ...(options.toon ? { toon: options.toon } : {}) });
	const head = b.add({ id: `g${gi}s${k}`, kind: 'slice', status: reach.slices[gi]?.[k] ?? 'unreachable', group: gi, slice: k, path: ['groups', gi, 'slices', k], name: `${k + 1}`, ...(image ? { image } : {}) });
	b.edge(group, head, 'then');
	// the group's script events that reveal one of this slice's text boxes
	const boxes = new Set<string>((Array.isArray(slice.dialog) ? slice.dialog : []).map((d: any) => d?.id).filter((id: unknown): id is string => typeof id === 'string'));
	(Array.isArray(g.events) ? g.events : []).forEach((e: any, j: number) => {
		if (e?.type !== 'showTextBox' || !boxes.has(e.params?.dialog)) return;
		const path = ['groups', gi, 'events', j];
		const at = b.add({ id: `g${gi}e${j}`, kind: conditional(e) ? 'step' : 'run', status: b.statusAt(path, reach.events[gi]?.[j] ?? 'unreachable'), group: gi, path, events: [j, j], boxes: [String(e.params.dialog)], stub: true, ...(conditional(e) ? { event: 'showTextBox', condition: conditionText(e.if) } : {}) });
		b.edge(group, at, 'then');
		b.edge(at, head, 'reveal', b.cond(e));
	});
	b.triggers(gi, { detail: true, from: () => head, only: k, points: true });
	return b.done();
}

// ── Season / toon level ─────────────────────────────────────────────────────────────────────────────────────────

/** a chapter of the toon as the season / toon level reads it */
export interface FlowChapter {
	number: number;
	/** its groups with each scene in place of its ref */
	groups: readonly any[];
	/** its status in its season (source language): draft / coming-soon / published */
	status?: string;
	title?: string;
	/** its thumbnail (`Chapter.thumbnail`, relative to the toon's folder) */
	thumbnail?: string;
}
/** a season and its chapters in reading order (`sequence`) */
export interface FlowSeason {
	number: number;
	chapters: number[];
}
/** a map place that leads into a chapter of this toon (hidden = unlocked by reading, not an entry by itself) */
export interface FlowPlace {
	id: string;
	name: string;
	chapter: number;
	hidden?: boolean;
}
export interface ToonFlowInput {
	toon: string;
	chapters: readonly FlowChapter[];
	seasons: readonly FlowSeason[];
	/** the toon's facts (`toonFacts` over its documents + the map) */
	facts?: ToonFacts;
	places?: readonly FlowPlace[];
	/** chapters of this toon other toons jump into (`incomingJumps` over the server's flow index): entries like a map place */
	incoming?: readonly number[];
	/** a text (a choice's label) as shown */
	text?: (value: unknown) => string;
}

/** a way out of a chapter: another chapter / season / toon … (`target`) or the next chapter of the season (`next`) */
export interface ChapterExit {
	target?: JumpToTarget;
	next?: boolean;
	kind: FlowEdgeKind;
	/** the choice's text (a choice that jumps) */
	label?: string;
	/** how sure the way is: reachable, only through a condition, or not at all */
	status: Reach;
}

/** what leaves a chapter (from its flow's exits; one entry per way, the best status of its edges) */
export function chapterExits(flow: ChapterFlow): ChapterExit[] {
	const byId = new Map(flow.nodes.map((n) => [n.id, n]));
	const out = new Map<string, ChapterExit>();
	for (const e of flow.edges) {
		const to = byId.get(e.to);
		if (to?.kind !== 'exit') continue;
		const r = e.never ? 0 : Math.min(RANK[e.status], e.condition ? 2 : 3);
		const status: Reach = r >= 3 ? 'reachable' : r >= 1 ? 'conditional' : 'unreachable';
		const kind = e.kind === 'follow' && to.next ? 'order' : e.kind;
		const key = `${to.id}|${kind}|${e.label ?? ''}`;
		const had = out.get(key);
		if (had && RANK[had.status] >= RANK[status]) continue;
		out.set(key, { ...(to.next ? { next: true } : { target: to.target }), kind, ...(e.label ? { label: e.label } : {}), status });
	}
	return [...out.values()];
}

export interface ToonChapterReach {
	/** reachable in the toon: the season order or a jump leads there (or the reader opens it from the overview) */
	reach: Reach;
	/** groups of the chapter nothing reaches (`chapterReach`) */
	dead: number;
	flow: ChapterFlow;
	exits: ChapterExit[];
}

/**
 * A toon's reach over its chapters (owner 2026-10-06): entries are every season's first chapter, the published chapters
 * (readable from the toon page's list) and chapters a visible map place opens; from a reached chapter its follow-up bar
 * leads to the next chapter of the season (when the chapter can get there), its jumps / ends to other chapters, a season
 * (its first chapter) or the toon page (every season's start). Iterative, cycles are harmless.
 */
export function toonReach(input: ToonFlowInput): Map<number, ToonChapterReach> {
	const out = new Map<number, ToonChapterReach>();
	for (const c of input.chapters) {
		const flow = chapterFlow(c.groups, { chapter: c.number, toon: input.toon, facts: input.facts, text: input.text });
		out.set(c.number, { reach: 'unreachable', dead: flow.reach.groups.filter((r) => r === 'unreachable').length, flow, exits: chapterExits(flow) });
	}
	const nextOf = new Map<number, number>();
	const starts: number[] = [];
	for (const s of input.seasons) {
		if (s.chapters.length) starts.push(s.chapters[0]);
		s.chapters.forEach((n, i) => i + 1 < s.chapters.length && nextOf.set(n, s.chapters[i + 1]));
	}
	const seasonStart = (n: number) => input.seasons.find((s) => s.number === n)?.chapters[0];
	const lead = (number: number, x: ChapterExit): number[] => {
		if (x.next) return nextOf.has(number) ? [nextOf.get(number)!] : [];
		const t = x.target;
		if (!t) return [];
		if (t.type === 'chapter') return intoOtherToon(t, input.toon) ? [] : [Number(t.value)];
		if (t.type === 'season') return [seasonStart(Number(t.value))].filter((n): n is number => n !== undefined);
		if (t.type === 'toon' && t.value === input.toon) return starts;
		return [];
	};
	const entries = new Set<number>(starts);
	for (const c of input.chapters) if (c.status === 'published') entries.add(c.number);
	for (const p of input.places ?? []) if (!p.hidden) entries.add(p.chapter);
	for (const n of input.incoming ?? []) entries.add(n);
	const walk = (withConditions: boolean): Set<number> => {
		const reached = new Set<number>();
		const queue = [...entries].filter((n) => out.has(n));
		queue.forEach((n) => reached.add(n));
		while (queue.length) {
			const n = queue.pop()!;
			for (const x of out.get(n)!.exits) {
				if (x.status === 'unreachable' || (x.status === 'conditional' && !withConditions)) continue;
				for (const to of lead(n, x)) if (out.has(to) && !reached.has(to)) (reached.add(to), queue.push(to));
			}
		}
		return reached;
	};
	const sure = walk(false);
	const maybe = walk(true);
	for (const [n, r] of out) r.reach = sure.has(n) ? 'reachable' : maybe.has(n) ? 'conditional' : 'unreachable';
	return out;
}

/**
 * The season level (`season` given: its chapters as boxes, the others as exits) or the toon level (every chapter, the
 * map places): chapter boxes with their status and reach, the season order, jumps / ends between chapters (parallel ways
 * merged, their texts on the edge) and exits to other seasons / toons / products.
 */
export function toonFlow(input: ToonFlowInput, options: { season?: number; reach?: Map<number, ToonChapterReach> } = {}): { nodes: FlowNode[]; edges: FlowEdge[]; reach: Map<number, ToonChapterReach> } {
	const reach = options.reach ?? toonReach(input);
	const nodes: FlowNode[] = [];
	const edges: FlowEdge[] = [];
	const byId = new Map<string, FlowNode>();
	const add = (n: FlowNode) => (byId.has(n.id) ? n.id : (byId.set(n.id, n), nodes.push(n), n.id));
	const seasonOf = new Map<number, number>();
	for (const s of input.seasons) for (const n of s.chapters) if (!seasonOf.has(n)) seasonOf.set(n, s.number);
	const shown = (n: number) => reach.has(n) && (options.season === undefined || seasonOf.get(n) === options.season);
	const ordered = [...input.chapters].sort((a, b) => (seasonOf.get(a.number) ?? 1e9) - (seasonOf.get(b.number) ?? 1e9) || a.number - b.number);
	for (const c of ordered) {
		if (!shown(c.number)) continue;
		const r = reach.get(c.number)!;
		add({
			id: `c${c.number}`,
			kind: 'chapter',
			status: r.reach,
			chapter: c.number,
			...(seasonOf.has(c.number) ? { season: seasonOf.get(c.number) } : {}),
			...(c.status ? { state: c.status } : {}),
			...(c.title ? { name: c.title } : {}),
			count: c.groups.length,
			dead: r.dead,
			...(c.thumbnail ? { image: { kind: 'thumbnail', path: c.thumbnail, chapter: c.number, toon: input.toon } as FlowImage } : {})
		});
	}
	/** merged edges: from|to|kind → edge */
	const merged = new Map<string, FlowEdge>();
	const link = (from: string, to: string, kind: FlowEdgeKind, status: Reach, label?: string) => {
		if (from === to) return;
		const key = `${from}|${to}|${kind}`;
		const had = merged.get(key);
		if (had) {
			had.count = (had.count ?? 1) + 1;
			if (label && !(had.label ?? '').split(' · ').includes(label)) had.label = had.label ? `${had.label} · ${label}` : label;
			if (RANK[status] > RANK[had.status]) had.status = status;
			return;
		}
		const e: FlowEdge = { id: `${from}>${to}#${edges.length}`, from, to, kind, status, ...(label ? { label } : {}) };
		merged.set(key, e);
		edges.push(e);
	};
	const exitNode = (t: JumpToTarget): string => {
		const other = intoOtherToon(t, input.toon);
		const id = `x:${t.type}:${other ? `${t.toon}/` : ''}${t.value}`;
		const target: JumpToTarget = { type: t.type, value: String(t.value), ...(other ? { toon: t.toon } : {}) };
		// a chapter of this toon in another season: its thumbnail from its document; else looked up by the creator
		const own = t.type === 'chapter' && !other ? input.chapters.find((c) => c.number === Number(t.value))?.thumbnail : undefined;
		const pictures = own ? { image: { kind: 'thumbnail', path: own, chapter: Number(t.value), toon: input.toon } as FlowImage } : targetPictures(target, input.toon);
		return add({ id, kind: 'exit', status: 'unreachable', target, ...pictures, ...(t.type === 'chapter' && !other && seasonOf.has(Number(t.value)) ? { season: seasonOf.get(Number(t.value)) } : {}) });
	};
	for (const c of ordered) {
		if (!shown(c.number)) continue;
		const r = reach.get(c.number)!;
		const from = `c${c.number}`;
		const fromStatus = (s: Reach): Reach => (r.reach === 'unreachable' ? 'unreachable' : r.reach === 'conditional' && s === 'reachable' ? 'conditional' : s);
		for (const x of r.exits) {
			if (x.status === 'unreachable') continue;
			if (x.next) {
				const s = input.seasons.find((s) => s.chapters.includes(c.number));
				const at = s ? s.chapters.indexOf(c.number) : -1;
				const next = s && at >= 0 ? s.chapters[at + 1] : undefined;
				if (next !== undefined && reach.has(next)) link(from, shown(next) ? `c${next}` : exitNode({ type: 'chapter', value: String(next) } as JumpToTarget), 'order', fromStatus(x.status));
				continue;
			}
			const t = x.target!;
			const label = x.label ?? t.label ?? (t.anyOf?.length ? t.anyOf.join(' / ') : undefined);
			let to: string;
			if (t.type === 'chapter' && !intoOtherToon(t, input.toon) && reach.has(Number(t.value))) to = shown(Number(t.value)) ? `c${Number(t.value)}` : exitNode(t);
			else if (t.type === 'season' && options.season === undefined) {
				const start = input.seasons.find((s) => s.number === Number(t.value))?.chapters[0];
				to = start !== undefined && shown(start) ? `c${start}` : exitNode(t);
			} else to = exitNode(t);
			link(from, to, x.kind, fromStatus(x.status), label);
		}
	}
	// the toon level: map places as entries (one box per chapter)
	if (options.season === undefined) {
		const byChapter = new Map<number, FlowPlace[]>();
		for (const p of input.places ?? []) if (shown(p.chapter)) byChapter.set(p.chapter, [...(byChapter.get(p.chapter) ?? []), p]);
		for (const [n, list] of byChapter) {
			const id = add({ id: `p${n}`, kind: 'place', status: list.some((p) => !p.hidden) ? 'reachable' : 'conditional', chapter: n, places: list.map((p) => p.name) });
			link(id, `c${n}`, 'place', byId.get(id)!.status as Reach);
		}
	}
	// an exit is as reachable as the best way into it
	for (const n of nodes) {
		if (n.kind !== 'exit') continue;
		const best = Math.max(0, ...edges.filter((e) => e.to === n.id).map((e) => RANK[e.status]));
		n.status = best >= 3 ? 'reachable' : best >= 1 ? 'conditional' : 'unreachable';
	}
	return { nodes, edges, reach };
}

// ── SCUTOONS level (the server's flow index) ────────────────────────────────────────────────────────────────────

/** one way somewhere, as the server's flow index lists it (`GET /api/creator/flow-index`) */
export interface FlowIndexEdge {
	source: { toon: string; chapter: number; group?: string | null };
	target: JumpToTarget;
	kind: 'jump' | 'choice' | 'end';
	label?: string;
}
export interface FlowIndexToon {
	id: string;
	title?: unknown;
	status?: string;
	seasons: { number: number; chapters: { number: number; status?: unknown }[] }[];
	/** its avatar (the toon entry's `avatar`, relative to the toon's folder) */
	avatar?: string | null;
	/**
	 * per chapter number: its thumbnail and, per group label, the low variant of the group's first slice (where a jump to
	 * that label lands) – the Flow's pictures of places in other chapters / toons (`chapterPictures`)
	 */
	chapters?: Record<string, ChapterPictures>;
}
/** a chapter's pictures as the flow index lists them (paths relative to the toon's folder) */
export interface ChapterPictures {
	thumbnail?: string;
	/** group label → its first slice's low variant */
	labels?: Record<string, string>;
	/** the first group's first slice (a jump to the chapter without a label lands there) */
	first?: string;
}

/** a chapter's pictures from its groups (scenes in place of their refs) and thumbnail – what the server's flow index lists */
export function chapterPictures(groups: readonly any[], thumbnail?: unknown): ChapterPictures {
	const labels: Record<string, string> = {};
	const low = (g: any) => (Array.isArray(g?.slices) ? sliceImage(g.slices[0], 0) : undefined);
	for (const g of groups) {
		if (typeof g?.label !== 'string' || !g.label || g.label in labels) continue;
		const image = low(g);
		if (image?.kind === 'slice') labels[g.label] = image.path;
	}
	const first = low(groups[0]);
	return {
		...(typeof thumbnail === 'string' && thumbnail ? { thumbnail } : {}),
		...(Object.keys(labels).length ? { labels } : {}),
		...(first?.kind === 'slice' ? { first: first.path } : {})
	};
}
export interface FlowIndex {
	toons: FlowIndexToon[];
	edges: FlowIndexEdge[];
	/** per toon: its dead things (the same rules as `deadThings`, variables judged across every toon) */
	dead?: Record<string, DeadThings>;
	/** every story variable of SCUTOONS by storage key (`catpope/x`, `x` = global – variables.ts): the toons that set it,
	 *  the toons that read it (one save game for all toons) */
	variables?: Record<string, { set: string[]; read: string[] }>;
}

/**
 * What the OTHER toons do with story variables (from the server's flow index): the storage keys they set and read – `toonFacts(…, { elsewhere })` (dead things, never-true tests and unknown names judged across all toons).
 */
export function variablesElsewhere(index: Pick<FlowIndex, 'variables'> | null | undefined, toon: string): { sets: string[]; reads: string[] } {
	const sets: string[] = [];
	const reads: string[] = [];
	for (const [name, v] of Object.entries(index?.variables ?? {})) {
		if ((v?.set ?? []).some((t) => t !== toon)) sets.push(name);
		if ((v?.read ?? []).some((t) => t !== toon)) reads.push(name);
	}
	return { sets: sets.sort(), reads: reads.sort() };
}

/** one box per toon (its seasons), the jumps between toons (merged, counted) and to products / XR */
export function universeFlow(index: FlowIndex, options: { text?: (value: unknown) => string } = {}): { nodes: FlowNode[]; edges: FlowEdge[] } {
	const text = options.text ?? firstText;
	const nodes: FlowNode[] = [];
	const edges: FlowEdge[] = [];
	const byId = new Map<string, FlowNode>();
	const add = (n: FlowNode) => (byId.has(n.id) ? n.id : (byId.set(n.id, n), nodes.push(n), n.id));
	for (const t of index.toons) {
		add({
			id: `t:${t.id}`,
			kind: 'toon',
			status: 'reachable',
			toon: t.id,
			name: text(t.title) || t.id,
			...(t.status ? { state: t.status } : {}),
			image: { kind: 'toon', toon: t.id },
			seasons: t.seasons.map((s) => s.number),
			count: t.seasons.reduce((sum, s) => sum + s.chapters.length, 0)
		});
	}
	const merged = new Map<string, FlowEdge>();
	for (const e of index.edges) {
		const from = `t:${e.source.toon}`;
		if (!byId.has(from)) continue;
		const t = e.target;
		let to: string;
		// a toon's page, or a chapter (+ label) of another toon
		const into = t.type === 'toon' ? String(t.value) : intoOtherToon(t, e.source.toon) ? String(t.toon) : null;
		const label = e.label;
		if (into !== null) {
			if (into === e.source.toon) continue;
			to = byId.has(`t:${into}`) ? `t:${into}` : add({ id: `x:toon:${into}`, kind: 'exit', status: 'reachable', target: { type: 'toon', value: into } as JumpToTarget, image: { kind: 'toon', toon: into } });
		} else if (t.type === 'product' || t.type === 'xr') {
			to = add({ id: `x:${t.type}:${t.value}`, kind: 'exit', status: 'reachable', target: { type: t.type, value: String(t.value) } });
		} else continue; // chapter / season: inside the toon
		const kind: FlowEdgeKind = e.kind;
		const key = `${from}|${to}|${kind}`;
		const had = merged.get(key);
		if (had) {
			had.count = (had.count ?? 1) + 1;
			if (label && !(had.label ?? '').split(' · ').includes(label)) had.label = had.label ? `${had.label} · ${label}` : label;
			continue;
		}
		const edge: FlowEdge = { id: `${from}>${to}#${edges.length}`, from, to, kind, status: 'reachable', ...(label ? { label } : {}), count: 1 };
		merged.set(key, edge);
		edges.push(edge);
	}
	return { nodes, edges };
}

/**
 * What other toons' jumps lead into `toon` (from the server's flow index): its chapters (entries of `toonReach`) and the
 * labels they name (entry points of `toonFacts` / `chapterReach`). A label in another toon counts as reached from there.
 */
export function incomingJumps(index: Pick<FlowIndex, 'edges'> | null | undefined, toon: string): { chapters: number[]; labels: string[] } {
	const chapters = new Set<number>();
	const labels = new Set<string>();
	for (const e of index?.edges ?? []) {
		const t = e.target;
		if (e.source.toon === toon || t?.type !== 'chapter' || t.toon !== toon) continue;
		if (Number.isFinite(Number(t.value))) chapters.add(Number(t.value));
		if (t.label) labels.add(t.label);
		for (const l of t.anyOf ?? []) if (l) labels.add(l);
	}
	return { chapters: [...chapters].sort((a, b) => a - b), labels: [...labels].sort() };
}

// ── Dead things ─────────────────────────────────────────────────────────────────────────────────────────────────

/** where something dead is: chapter, group (index), its label */
export interface DeadPlace {
	chapter: number;
	group: number;
	label?: string;
}
export interface DeadItem {
	name: string;
	at: DeadPlace[];
}
export interface DeadThings {
	/**
	 * variables something sets (setVariable, a choice's set / add, an askText's answer) that nothing reads – in no toon:
	 * a simple test, an expression condition (`if.expr`) or a text's `{…}` (the expression analyser's names of one part)
	 */
	unread: DeadItem[];
	/** variables tested or shown that nothing sets – in no toon (their tests are never true) */
	unset: DeadItem[];
	/** named points of a group's script nothing diverts to */
	points: DeadItem[];
	/** group labels nothing jumps to (no jump / end / random list / map place, and the follow-up bar doesn't lead there) */
	labels: DeadItem[];
}

/** the story variables an expression / a text's `{…}` reads, as written: `coins`, `global.coins`, `toptoy.coins`
 *  (`reader.name` and the other namespaces are no variables – variables.ts) */
const storyNames = (names: string[][]): string[] => names.filter(isStoryPath).map((n) => n.join('.'));

/**
 * The toon's dead things over its chapters (groups with each scene in place of its ref). `facts` = the toon's
 * (`toonFacts` over every document + the map, `elsewhere` = the other toons'): variables set / read elsewhere (item
 * pickups, other documents, other toons) and labels something refers to; without them no label is judged.
 */
export function deadThings(chapters: readonly { number: number; groups: readonly any[] }[], facts?: ToonFacts): DeadThings {
	// keyed by storage keys (variables.ts: `x` here and `catpope.x` elsewhere are one variable), shown as written here
	const sets = new Map<string, DeadPlace[]>();
	const reads = new Map<string, DeadPlace[]>();
	const note = (map: Map<string, DeadPlace[]>, name: string, at: DeadPlace) => {
		if (!name || name.startsWith('picked:')) return;
		const key = factKey(facts, name);
		const list = map.get(key) ?? [];
		if (!list.some((p) => p.chapter === at.chapter && p.group === at.group)) list.push(at);
		map.set(key, list);
	};
	const shown = (key: string) => (facts?.toon ? writtenVariable(key, facts.toon) : key);
	const points: DeadItem[] = [];
	const labels: DeadItem[] = [];
	for (const c of chapters) {
		c.groups.forEach((g, gi) => {
			const at: DeadPlace = { chapter: c.number, group: gi, ...(typeof g?.label === 'string' && g.label ? { label: g.label } : {}) };
			const named: string[] = [];
			const diverted = new Set<string>();
			// every object of the group once: events, choices, slices, items, text boxes
			const stack: unknown[] = [g];
			while (stack.length) {
				const v = stack.pop();
				if (!v || typeof v !== 'object') {
					if (typeof v === 'string' && v.includes('{')) for (const name of storyNames(analyseText(v).reads.names)) note(reads, name, at);
					continue;
				}
				if (Array.isArray(v)) {
					for (const x of v) stack.push(x);
					continue;
				}
				const o = v as Record<string, any>;
				if (o.type === 'setVariable' && typeof o.params?.variable === 'string') note(sets, o.params.variable, at);
				const asked = askVariable(o);
				if (asked) note(sets, asked, at);
				if (typeof o.set?.variable === 'string') note(sets, o.set.variable, at);
				if (typeof o.add?.variable === 'string') note(sets, o.add.variable, at);
				if (typeof o.if?.variable === 'string') note(reads, o.if.variable, at);
				if (typeof o.if?.expr === 'string') for (const name of storyNames(analyseExpression(o.if.expr).reads.names)) note(reads, name, at);
				if (o.type === 'label' && typeof o.params?.name === 'string' && o.params.name) named.push(o.params.name);
				if (o.type === 'divert' && typeof o.params?.to === 'string') diverted.add(o.params.to);
				if (typeof o.divert?.to === 'string') diverted.add(o.divert.to);
				for (const key in o) if (key !== 'if') stack.push(o[key]);
			}
			for (const name of new Set(named)) if (!diverted.has(name)) points.push({ name, at: [at] });
			// a group label: something refers to it, it starts the chapter, or the follow-up bar leads there
			if (facts && at.label && gi > 0 && !facts.labels.has(at.label)) {
				const before = c.groups[gi - 1];
				const bar = !before?.continues && !groupEnd(before);
				if (!bar) labels.push({ name: at.label, at: [at] });
			}
		});
	}
	const setAnywhere = (name: string) => sets.has(name) || !!facts?.variables.has(name);
	const sorted = (list: DeadItem[]) => list.sort((a, b) => a.name.localeCompare(b.name));
	return {
		unread: sorted([...sets].filter(([key]) => !reads.has(key) && !facts?.reads?.has(key)).map(([key, at]) => ({ name: shown(key), at }))),
		unset: sorted([...reads].filter(([key]) => !setAnywhere(key)).map(([key, at]) => ({ name: shown(key), at }))),
		points,
		labels
	};
}
