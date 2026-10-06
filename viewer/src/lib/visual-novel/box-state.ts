/**
 * The VN box's waiting state – ONE state machine for everything that can make the reader wait (owner 2026-10-06:
 * "Events queue takeovers have to be safe for accidental dead ends" – by logic, not by timers): the message and its
 * click, the choices, an askText question, slice-trigger runs (one at a time) that may park the script's waiting message,
 * the page's scripts. Pure (no Svelte, no DOM): the VN context (`contexts/visual-novel.svelte.ts`) passes its reactive
 * `$state` object as `data`, the viewer's node tests (test/vn-safety.test.ts) a plain one – the same code runs.
 *
 * Every waiting state has an exit the reader can take (`answerable`):
 *   - a message waiting for its click → its continue overlay (a parked message shows no overlay: its click belongs to
 *     the script, which gets it back when the trigger run ends – `endRun`, also after an error)
 *   - choices → shown only when there is at least one (`showChoices` with none: the script goes on – no freeze)
 *   - a question → its input line, ✓ and Skip (Skip always: the default or nothing)
 *   - a text box with `wait` → the tap layer (and its effect ends by itself)
 * Slice-trigger runs take the box in turn (`acquireRun`, first come first served, and only while no question / choices
 * are open): two runs never park the same message or write one resolver over another. While a run holds the box the
 * page's script pauses before its next event (`whenNoRun` – the runner), so the two never write over each other's
 * message; a run waiting for its turn holds nothing (no deadlock: it waits for the reader, never for the script). The
 * page's scripts start through the script queue when the box is `idle` (script-queue.ts, woken by `subscribe`).
 * `frozen` (the page doesn't scroll) = something answerable, or a run / script in progress – in a settled state the
 * second implies the first (test/vn-safety.test.ts walks random scripts, triggers and readers to show it).
 */

export interface BoxMessage<C> {
	character: C | null;
	text: string;
	wait: boolean;
}

export interface BoxQuestion {
	placeholder: string;
	maxLength: number;
	/** what Skip stores (shown on the button's title) */
	fallback: string;
	resolve: (answer: string | null) => void;
}

/** the reactive part (the context's `$state`) */
export interface BoxData<C, Ch> {
	messageVisible: boolean;
	message: BoxMessage<C>;
	choicesVisible: boolean;
	pendingChoices: Ch[];
	question: BoxQuestion | null;
	/** slice-trigger / placed-item runs started and not ended (waiting for their turn included) */
	runs: number;
	/** the page's scripts running or in line (script-queue.ts) */
	pageBusy: number;
	/** every script the page must run has finished */
	pageDone: boolean;
	/** the group's script ran to its end without navigating */
	eventsDone: boolean;
}

export function createBoxData<C, Ch>(): BoxData<C, Ch> {
	return {
		messageVisible: false,
		message: { character: null, text: '', wait: false },
		choicesVisible: false,
		pendingChoices: [],
		question: null,
		runs: 0,
		pageBusy: 0,
		pageDone: true,
		eventsDone: false
	};
}

export interface BoxOptions {
	/** a text box with `wait` plays (its tap layer continues) */
	textBoxWaiting?: () => boolean;
}

export type Box<C, Ch> = ReturnType<typeof createBox<C, Ch>>;

export function createBox<C, Ch>(data: BoxData<C, Ch>, options: BoxOptions = {}) {
	/** the waiting message's click (the script / run that showed it awaits it) */
	let waitResolver: (() => void) | null = null;
	/** the reader clicked the visible message away – a following `jumpTo` (wait) jumps at once */
	let acknowledged = false;
	/** the script's waiting message while a trigger run uses the box (`interrupt`) – given back by `endRun` */
	let suspended: { message: BoxMessage<C>; visible: boolean; resolver: () => void; acknowledged: boolean; by: number } | null = null;
	let navigationTriggered = false;
	let pendingDivert: string | null = null;
	/** trigger runs take turns: the one holding the box (0 = none), the ones waiting (in order) */
	let holder = 0;
	let nextRun = 0;
	const turns: number[] = [];
	/** runs a page reset let go: their `endRun` changes nothing any more */
	const gone = new Set<number>();
	const listeners = new Set<() => void>();

	function changed() {
		for (const fn of [...listeners]) fn();
	}
	/** resolves once `test()` holds (checked now and after every change) */
	function when(test: () => boolean): Promise<void> {
		if (test()) return Promise.resolve();
		return new Promise((resolve) => {
			const check = () => {
				if (!test()) return;
				listeners.delete(check);
				resolve();
			};
			listeners.add(check);
		});
	}

	// ── the message ──────────────────────────────────────────────────────────────────────────────────────────────
	function showMessage(character: C | null, text: string, wait: boolean) {
		data.message.character = character;
		data.message.text = text;
		data.message.wait = wait;
		data.messageVisible = true;
		acknowledged = false;
		changed();
	}
	function hideMessage() {
		data.messageVisible = false;
		changed();
	}
	/** the shown message waits for its click: resolves on `continueFromWait` */
	function waitForClick(): Promise<void> {
		return new Promise<void>((resolve) => {
			waitResolver = resolve;
			data.message.wait = true;
			changed();
		});
	}
	function continueFromWait() {
		if (!waitResolver) return;
		// the click answers this message: no "…" any more, and nothing else waits for the same click
		data.message.wait = false;
		acknowledged = true;
		const resolve = waitResolver;
		waitResolver = null;
		changed();
		resolve();
	}
	/**
	 * `jumpTo` with `wait`: jump once the reader is done with the box. When the reader already clicked the last message
	 * away (or no message is shown, or choices are open) at once; else the message gets the "…" and one click.
	 */
	function waitForContinue(): Promise<void> {
		if (acknowledged || !data.messageVisible || data.choicesVisible) return Promise.resolve();
		return waitForClick();
	}

	// ── the group's script ───────────────────────────────────────────────────────────────────────────────────────
	function beginEvents() {
		data.eventsDone = false;
	}
	/** the script's events are done: a message the reader already clicked away closes (the last click advances too) */
	function endOfEvents() {
		if (acknowledged && !data.choicesVisible) data.messageVisible = false;
		data.eventsDone = true;
		changed();
	}

	// ── askText ──────────────────────────────────────────────────────────────────────────────────────────────────
	function ask(q: Omit<BoxQuestion, 'resolve'>): Promise<string | null> {
		data.question?.resolve(null);
		return new Promise((resolve) => {
			data.question = { ...q, resolve };
			changed();
		});
	}
	function answer(value: string | null) {
		const q = data.question;
		data.question = null;
		changed();
		q?.resolve(value);
	}

	// ── choices ──────────────────────────────────────────────────────────────────────────────────────────────────
	function addChoice(choice: Ch) {
		data.pendingChoices.push(choice);
		changed();
	}
	/** choices a script added but never showed (its run ended): gone, so they can't join a later question */
	function dropUnshownChoices(count: number) {
		if (data.choicesVisible || count <= 0) return;
		data.pendingChoices.splice(Math.max(0, data.pendingChoices.length - count), count);
		changed();
	}
	/**
	 * Shows the added choices – only when there is one (every choice hidden by its condition: nothing to ask, the script
	 * goes on; a question without a choice would freeze the page with nothing to answer). Returns whether it asks.
	 */
	function showChoices(): boolean {
		if (!data.pendingChoices.length) {
			data.choicesVisible = false;
			changed();
			return false;
		}
		data.choicesVisible = true;
		changed();
		return true;
	}
	/** resolves once the reader picked (the choices are gone) */
	function whenChoicesClosed(): Promise<void> {
		return when(() => data.pendingChoices.length === 0);
	}
	function hideChoices() {
		data.choicesVisible = false;
		changed();
	}
	/** a choice was picked: the question goes, a parked message may come back */
	function clearChoices() {
		data.pendingChoices.length = 0;
		data.choicesVisible = false;
		data.messageVisible = false;
		changed();
		resume();
	}
	function setPendingDivert(to: string | null) {
		pendingDivert = to;
	}
	function takePendingDivert(): string | null {
		const to = pendingDivert;
		pendingDivert = null;
		return to;
	}
	function setNavigationTriggered() {
		navigationTriggered = true;
	}

	// ── trigger runs: one at a time, parking the script's waiting message ─────────────────────────────────────────
	/** nothing for the reader to answer is open (no question, no choices added or shown) */
	const free = () => !data.choicesVisible && !data.pendingChoices.length && !data.question;
	/**
	 * A run starts (the page stays frozen meanwhile); resolves with its id once it holds the box: no other run holds it,
	 * it is first in line, and nothing for the reader is open (a question / choices are answered first).
	 */
	async function acquireRun(): Promise<number> {
		const id = ++nextRun;
		data.runs++;
		turns.push(id);
		changed();
		// checked again after every wake-up: another continuation (a script's next event) may have opened something in
		// between – taking the box is only ever decided in one synchronous step
		const ready = () => gone.has(id) || (holder === 0 && turns[0] === id && free());
		while (!ready()) await when(ready);
		if (!gone.has(id)) {
			turns.shift();
			holder = id;
			changed();
		}
		return id;
	}
	/** the run holds the box (false: a page reset let it go while it waited) */
	function holds(id: number): boolean {
		return holder === id;
	}
	/**
	 * no run holds the box – the page's script goes on with its next event. Use it as
	 * `while (box.runHolds()) await box.whenNoRun()`: a run may take the box between the wake-up and the script's next step
	 */
	function whenNoRun(): Promise<void> {
		return when(() => holder === 0);
	}
	const runHolds = () => holder !== 0;
	/**
	 * The run `by` takes the box while the script's message waits for its click: the message (text + its wait) is parked
	 * – shown without the "…", its click is the script's – until `endRun(by)` gives it back.
	 */
	function interrupt(by: number) {
		if (suspended || !waitResolver || !data.messageVisible) return;
		suspended = { message: { ...data.message }, visible: data.messageVisible, resolver: waitResolver, acknowledged, by };
		waitResolver = null;
		data.message.wait = false;
		changed();
	}
	/**
	 * the parked message back – once the run that parked it is over (it no longer holds the box) and nothing of it is
	 * open (no choices, no question, no message of its own waiting): a run's question answered mid-run doesn't bring it
	 * back early (the run's next message would take its click)
	 */
	function resume() {
		if (!suspended || holder === suspended.by || data.choicesVisible || data.pendingChoices.length || data.question || waitResolver) return;
		data.message.character = suspended.message.character;
		data.message.text = suspended.message.text;
		data.message.wait = suspended.message.wait;
		data.messageVisible = suspended.visible;
		waitResolver = suspended.resolver;
		acknowledged = suspended.acknowledged;
		suspended = null;
		changed();
	}
	/** the run ended (done, navigated, cancelled or failed – call it in `finally`): its parked message back, the next run's turn */
	function endRun(id: number) {
		if (gone.delete(id)) return;
		data.runs = Math.max(0, data.runs - 1);
		if (holder === id) holder = 0;
		if (suspended?.by === id) {
			// its own message's click can't come any more (the run is over): drop that resolver, the script's is back
			waitResolver = null;
			resume();
		}
		const at = turns.indexOf(id);
		if (at >= 0) turns.splice(at, 1);
		changed();
	}

	// ── the page ─────────────────────────────────────────────────────────────────────────────────────────────────
	function pageState(state: { busy: number; done: boolean }) {
		data.pageBusy = state.busy;
		data.pageDone = state.done;
		changed();
	}

	/** what the reader can answer now (an element on screen that moves the story on) */
	function answerable(): boolean {
		if (data.question) return true;
		if (data.choicesVisible && data.pendingChoices.length > 0) return true;
		if (data.messageVisible && data.message.wait && !!waitResolver) return true;
		return !!options.textBoxWaiting?.();
	}
	/** nothing waits for the reader – the next script of the page may start (script-queue.ts `idle`) */
	function idle(): boolean {
		return !data.choicesVisible && !data.pendingChoices.length && !data.question && !(data.messageVisible && data.message.wait) && !data.runs;
	}
	/** the page doesn't scroll: something to answer, or a script / run in progress */
	function frozen(): boolean {
		return answerable() || data.pageBusy > 0 || data.runs > 0;
	}

	/** a fresh page: everything goes (a script awaiting the box is cancelled by its page) */
	function reset() {
		acknowledged = false;
		waitResolver = null;
		suspended = null;
		pendingDivert = null;
		navigationTriggered = false;
		// runs still going (their page went away) end without effect; waiting ones start nowhere
		for (let id = 1; id <= nextRun; id++) if (id === holder || turns.includes(id)) gone.add(id);
		holder = 0;
		turns.length = 0;
		const q = data.question;
		data.question = null;
		q?.resolve(null);
		data.pendingChoices.length = 0;
		data.choicesVisible = false;
		data.messageVisible = false;
		data.message.character = null;
		data.message.text = '';
		data.message.wait = false;
		data.eventsDone = false;
		data.pageBusy = 0;
		data.pageDone = true;
		data.runs = 0;
		changed();
	}

	return {
		data,
		showMessage,
		hideMessage,
		waitForClick,
		continueFromWait,
		waitForContinue,
		beginEvents,
		endOfEvents,
		ask,
		answer,
		addChoice,
		dropUnshownChoices,
		showChoices,
		whenChoicesClosed,
		hideChoices,
		clearChoices,
		setPendingDivert,
		takePendingDivert,
		setNavigationTriggered,
		isNavigationTriggered: () => navigationTriggered,
		acquireRun,
		holds,
		whenNoRun,
		runHolds,
		interrupt,
		resume,
		endRun,
		pageState,
		answerable,
		idle,
		frozen,
		/** the parked message (tests) */
		get parked() {
			return !!suspended;
		},
		/** a message's click is awaited (tests) */
		get awaitingClick() {
			return !!waitResolver;
		},
		/** a run holds the box (tests) */
		get holder() {
			return holder;
		},
		/** called after every change (the script queue wakes up) – returns the unsubscribe */
		subscribe(fn: () => void): () => void {
			listeners.add(fn);
			return () => listeners.delete(fn);
		},
		reset
	};
}
