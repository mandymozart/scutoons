// The event (research/EVENTS.md §14) and the story adapter (#236): `fromStory` turns a story event (shared/content
// visual-novel.ts `VNEvent`, structurally – no dependency on content) into the event deck-events (Rust) reads. The same
// rules in both, kept identical by test/fixtures/events.json (written by `cargo test -p deck-events`): the name is
// `story.<type>`, the event a **trigger** (`start: 'now'`, no steps); numeric params → `props` (keys sorted, the first
// six – deck-events' MAX_PROPS), the rest → `data`; `duration` (ms) → `dur`; the story's `id` stays. The story keeps its
// own runner: `if` is evaluated by the caller first, `wait`, choices, labels and jumps stay there (→ `null`).

export type EventTime = 'now' | { beats: number } | { seconds: number };

export interface DeckEvent {
	start: EventTime;
	/** none = a moment */
	dur?: EventTime;
	/** `<domain>.<name>`: `story.showMessage`, `relay.cue`, `midi.note` */
	name: string;
	/** fast numbers (real-time safe on the Rust side) */
	props?: Record<string, number>;
	/** loose: text, ids, references – never big payloads */
	data?: Record<string, unknown>;
	children?: DeckEvent[];
	id?: string;
	source?: string;
}

/** a story event as the chapter JSON has it (a slice event also carries a `trigger`, which the caller already used) */
export interface StoryEventLike {
	type: string;
	params?: Record<string, unknown>;
	id?: string;
}

/** the story types only its own runner handles (deck-events adapters/story.rs STAYS_IN_THE_STORY) */
export const STAYS_IN_THE_STORY: readonly string[] = ['showChoices', 'addChoice', 'label', 'divert', 'jumpTo', 'setVariable', 'askText'];
/** fast props per event (deck-events MAX_PROPS) */
export const MAX_PROPS = 6;
/** bytes of serialised `data` an adapter lets through (deck-events MAX_DATA): "send a reference, not the payload" */
export const MAX_DATA = 1024;

const NAME = /^[a-z][a-z0-9_]*(\.[a-z][A-Za-z0-9_]*)+$/;
const KEY = /^[a-z0-9_]+$/;

/** the byte length of `s` in UTF-8 (what Rust counts) */
const bytes = (s: string): number => new TextEncoder().encode(s).length;

export function fromStory(e: StoryEventLike, opts: { source?: string } = {}): DeckEvent | null {
	if (typeof e?.type !== 'string') throw new Error('a story event needs a string `type`');
	if (STAYS_IN_THE_STORY.includes(e.type)) return null;
	const name = `story.${e.type}`;
	if (!NAME.test(name)) throw new Error(`event name '${name}': only letters, 0–9 and _ in its parts (the domain in lower case)`);
	const event: DeckEvent = { start: 'now', name };
	const params = e.params;
	if (params !== undefined && (typeof params !== 'object' || params === null || Array.isArray(params))) {
		throw new Error(`${e.type}: \`params\` is not an object`);
	}
	if (params) {
		const ms = typeof params.duration === 'number' ? params.duration : undefined;
		const props: Record<string, number> = {};
		const data: Record<string, unknown> = {};
		for (const k of Object.keys(params).sort()) {
			if (k === 'wait' || (k === 'duration' && ms !== undefined)) continue;
			const v = params[k];
			if (typeof v === 'number' && KEY.test(k) && Object.keys(props).length < MAX_PROPS) props[k] = v;
			else data[k] = v;
		}
		if (Object.keys(props).length) event.props = props;
		if (Object.keys(data).length) {
			const n = bytes(JSON.stringify(data));
			if (n > MAX_DATA) throw new Error(`${name}: ${n} bytes of data (max ${MAX_DATA}) – send a reference, not the payload`);
			event.data = data;
		}
		if (ms !== undefined) event.dur = { seconds: ms / 1000 };
	}
	if (typeof e.id === 'string') event.id = e.id;
	if (opts.source !== undefined) event.source = opts.source;
	return event;
}
