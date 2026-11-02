// gate.ts (#298, owner 2026-11-02: "the start the sound button could be just a play button in the middle of the screen, or the
// first user clicking on anything in the scene. it is mostly about first-click blocking by the browser"): the host's small
// start gate. A Play button and the first user gesture on the host element both start the sound – the browser only lets an
// AudioContext run after a gesture, so the gesture itself must start it (synchronously, in the handler) and must not be
// swallowed: the listeners only observe (capture, passive, no preventDefault / stopPropagation). Pure state machine, no DOM
// beyond an EventTarget.

/** idle → starting → running | blocked (the browser or the engine refused: the Play button stays, a new gesture tries again) */
export type GateState = 'idle' | 'starting' | 'running' | 'blocked';
export type GateEvent = 'gesture' | 'started' | 'refused';

export function gateNext(state: GateState, event: GateEvent): GateState {
	switch (event) {
		case 'gesture':
			return state === 'idle' || state === 'blocked' ? 'starting' : state;
		case 'started':
			return state === 'starting' ? 'running' : state;
		case 'refused':
			return state === 'starting' ? 'blocked' : state;
	}
}

/** the gate listens (and shows its Play button) until the sound runs */
export const gateOpen = (s: GateState): boolean => s !== 'running';

/** the first gesture on the element: a press, a touch released, a key */
export const GATE_EVENTS = ['pointerdown', 'touchend', 'keydown'] as const;

export class StartGate {
	state: GateState = 'idle';
	private readonly running: (() => void)[] = [];

	/** `start` is the host's: it must create its AudioContext synchronously (inside the gesture) and answer whether it runs */
	constructor(
		private readonly start: () => Promise<boolean>,
		private readonly onState: (s: GateState) => void = () => undefined
	) {}

	private set(s: GateState): void {
		if (s === this.state) return;
		this.state = s;
		this.onState(s);
		if (s === 'running') for (const f of this.running.splice(0)) f();
	}

	/** a user gesture (or the Play button): starts the sound – nothing while it starts or runs */
	gesture(): Promise<void> {
		if (gateNext(this.state, 'gesture') === this.state) return Promise.resolve();
		this.set('starting');
		return this.start().then(
			(ok) => this.set(gateNext(this.state, ok ? 'started' : 'refused')),
			() => this.set(gateNext(this.state, 'refused'))
		);
	}

	/** the sound runs (it was started some other way, e.g. the page's own Start button) */
	markRunning(): void {
		this.set('running');
	}

	/** `f` once, when the sound runs */
	whenRunning(f: () => void): void {
		if (this.state === 'running') f();
		else this.running.push(f);
	}
}

/** Listen on `target` for the first gesture; gone once the sound runs. Returns the removal (also for a page that goes away). */
export function listenForGesture(target: EventTarget, gate: StartGate): () => void {
	const handler = () => void gate.gesture();
	const options = { capture: true, passive: true } as const;
	for (const type of GATE_EVENTS) target.addEventListener(type, handler, options);
	const off = () => {
		for (const type of GATE_EVENTS) target.removeEventListener(type, handler, options);
	};
	gate.whenRunning(off);
	return off;
}
