/**
 * The scripts of one reader page (owner 2026-10-06, agents/DATA.md "Linked groups"): linked groups share a page and
 * each may have a VN script. They share the one VN box, so they run ONE AFTER ANOTHER: a group asks to start
 * (`request`) when its first slice is seen; it starts once no other script of the page runs and the reader isn't
 * awaited (`idle`: no message waiting for its click, no choice open) – else it waits in line. The page is done
 * (`done`) when every script it must run (`register(…, required)`) has finished and nothing runs or waits.
 *
 * Pure (no Svelte, no DOM): the page component (`toon/interactions/vn-page.svelte`) wires it to the VN context; the
 * viewer's node test (test/script-queue.test.ts) drives it with fake scripts.
 */

export type ScriptResult = 'done' | 'navigated' | 'cancelled';

export interface ScriptQueueOptions {
	/** nothing waits for the reader (no message waiting for its click, no choice open) */
	idle: () => boolean;
	/** something changed (busy / done) – the page mirrors it into its reactive state */
	onChange?: (state: { busy: number; done: boolean; running: string | null }) => void;
	/** how often a waiting start looks at `idle` again (ms) */
	poll?: number;
	/** the timer (tests pass their own) */
	setTimer?: (fn: () => void, ms: number) => unknown;
	clearTimer?: (handle: unknown) => void;
}

export interface ScriptQueue {
	/** a script of the page; `required`: the page isn't done before it ran (the groups from the shown one on) */
	register(id: string, required: boolean): void;
	/** the group's script wants to start (its first slice was seen); once per id – later requests are ignored */
	request(id: string, run: () => Promise<ScriptResult>): void;
	/** the page went away: nothing starts any more */
	cancel(): void;
	/** scripts running or waiting to start */
	readonly busy: number;
	/** every required script finished, nothing runs or waits */
	readonly done: boolean;
	/** the running script's id */
	readonly running: string | null;
	/** the ids in the order they started */
	readonly started: readonly string[];
}

export function createScriptQueue(options: ScriptQueueOptions): ScriptQueue {
	const poll = options.poll ?? 100;
	const setTimer = options.setTimer ?? ((fn, ms) => setTimeout(fn, ms));
	const clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));
	const required = new Set<string>();
	const requested = new Set<string>();
	const finished = new Set<string>();
	const waiting: { id: string; run: () => Promise<ScriptResult> }[] = [];
	const started: string[] = [];
	let running: string | null = null;
	let timer: unknown = null;
	let cancelled = false;

	const busy = () => waiting.length + (running ? 1 : 0);
	const done = () => !cancelled && busy() === 0 && [...required].every((id) => finished.has(id));
	const changed = () => options.onChange?.({ busy: busy(), done: done(), running });

	function pump() {
		if (cancelled || running || !waiting.length) return;
		if (!options.idle()) {
			// the reader is awaited (a trigger's message, a choice): look again shortly
			if (timer === null)
				timer = setTimer(() => {
					timer = null;
					pump();
				}, poll);
			return;
		}
		const next = waiting.shift()!;
		running = next.id;
		started.push(next.id);
		changed();
		let result: Promise<ScriptResult>;
		try {
			result = next.run();
		} catch (error) {
			result = Promise.reject(error);
		}
		result
			.catch((error) => {
				console.error(error);
				return 'done' as ScriptResult;
			})
			.then(() => {
				if (running === next.id) running = null;
				finished.add(next.id);
				changed();
				pump();
			});
	}

	return {
		register(id, isRequired) {
			if (isRequired) required.add(id);
			else required.delete(id);
			changed();
		},
		request(id, run) {
			if (cancelled || requested.has(id)) return;
			requested.add(id);
			waiting.push({ id, run });
			changed();
			pump();
		},
		cancel() {
			cancelled = true;
			waiting.length = 0;
			running = null;
			if (timer !== null) clearTimer(timer);
			timer = null;
			changed();
		},
		get busy() {
			return busy();
		},
		get done() {
			return done();
		},
		get running() {
			return running;
		},
		get started() {
			return started;
		}
	};
}
