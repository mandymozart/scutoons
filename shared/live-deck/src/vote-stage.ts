// vote-stage.ts (#549, research/SHOW.md §15 point 3b): the phone's voting screen in an element – the browser deck's
// audience start on its light start (deck/space-web `VoteView`: deck-space's `audience` and `stage::Light`). The core
// joins, draws the ballot and what shows between votes, and casts; here only what the deck cannot reach on a phone: the
// canvas, its size and the animation frames, the socket (client.ts `RelaySocket`), the proof's fetch with the reader's
// sign-in (deck-key.ts `fetchProof`), the pointer and the keys passed on whole, and the core's word on whether the
// screen should stay on, handed to the page (its wake lock).
import { spaceModule, stageCanvas } from './space.ts';
import { RelaySocket, type RelayCoreView } from './client.ts';
import { fetchProof, type SignedIn } from './deck-key.ts';

/** #549: the wasm-bindgen API of deck/space-web's `VoteView` past the relay client's (`RelayCoreView`'s socket side) */
export interface VoteView {
	/** key (`r:` the proof), URL, … */
	asks(): unknown[];
	proof(status: number, body: string): void;
	proof_again(): void;
	/** press: 0 the pointer moved, 1 a press */
	pointer(x: number, y: number, press: number): boolean;
	key(key: string): boolean;
	awake(): boolean;
	backend(): string;
	resize(w: number, h: number): void;
	frame(time: number): void;
	free(): void;
}

export interface VoteOptions {
	host: HTMLElement;
	/** the URL of the folder with space_web.js and space_web_bg.wasm (ends with /) */
	base: string;
	/** the API (the proof's route) */
	api: string;
	/** `ws://host:port/live/<code>` ("" = the hosted show) */
	relay: string;
	/** the QR's `?group=` ("" = none) */
	group: string;
	locale: string;
	/** the signed-in reader's name ("" = nobody) */
	reader?: string;
	prefer?: 'auto' | 'webgl2';
	maxScale?: number;
	/** the viewer's signed-in user, if any (the cookie, or a bearer token) – the proof is asked with it */
	user?: () => SignedIn | null;
	/** the core's word changed: a vote can come (keep the screen on) or not */
	onAwake?: (awake: boolean) => void;
}

/** the mark of the first drawn frame (`performance.getEntriesByName`): the screen's first paint, measured on a phone */
export const FIRST_FRAME = 'vote-first-frame';

/** #549: the voting screen in `host` */
export class VoteStage {
	stopped = false;
	private raf = 0;
	private drawn = false;
	private awake = false;
	private guest = true;
	private readonly size: ResizeObserver;
	private readonly relay: RelaySocket;

	private constructor(
		private readonly o: VoteOptions,
		private readonly canvas: HTMLCanvasElement,
		readonly view: VoteView
	) {
		this.size = new ResizeObserver(() => this.fit());
		this.size.observe(o.host);
		this.fit();
		canvas.addEventListener('pointerdown', this.pointer);
		canvas.addEventListener('pointermove', this.pointer);
		addEventListener('keydown', this.key);
		(this.relay = new RelaySocket(view as unknown as RelayCoreView, {})).start();
		this.raf = requestAnimationFrame(this.frame);
	}

	/** Load the wasm, make the view (WebGPU, else WebGL2 on a fresh canvas), join and start drawing. */
	static async start(o: VoteOptions): Promise<VoteStage> {
		const mod = await spaceModule(o.base);
		const make = (canvas: HTMLCanvasElement, prefer: string) => mod.VoteView.create(canvas, o.relay, o.group, o.reader ?? '', o.locale, o.api, prefer);
		let canvas = stageCanvas(o);
		let view: VoteView;
		try {
			view = await make(canvas, o.prefer ?? 'auto');
		} catch (e) {
			if (o.prefer === 'webgl2') throw e;
			console.warn('vote: WebGPU failed, WebGL2 instead', e);
			canvas.remove();
			canvas = stageCanvas(o);
			view = await make(canvas, 'webgl2');
		}
		return new VoteStage(o, canvas, view);
	}

	private fit() {
		const k = Math.min(devicePixelRatio || 1, this.o.maxScale ?? 2);
		this.view.resize(this.canvas.clientWidth * k, this.canvas.clientHeight * k);
	}

	private frame = (ms: number) => {
		this.raf = requestAnimationFrame(this.frame);
		this.view.frame(ms / 1000);
		if (!this.drawn) {
			this.drawn = true;
			performance.mark(FIRST_FRAME);
		}
		// what the relay client says goes out; a sign-in or sign-out asks its proof again; what it asks for is fetched
		this.relay.run();
		const user = this.o.user?.() ?? undefined;
		if (!user !== this.guest) this.view.proof_again();
		this.guest = !user;
		const asked = this.view.asks();
		for (let i = 0; i + 1 < asked.length; i += 2) if (asked[i] === 'r:') void this.proof(asked[i + 1] as string, user);
		const awake = this.view.awake();
		if (awake !== this.awake) this.o.onAwake?.((this.awake = awake));
	};

	/** the proof, fetched with the reader's sign-in; no answer is an answer too (the core asks again later) */
	private async proof(url: string, user: SignedIn | undefined) {
		const view = { outlet_proof: (status: number, body: string) => void (this.stopped || this.view.proof(status, body)) };
		await fetchProof({ view, stopped: false }, { api: this.o.api, user }, url).catch(() => view.outlet_proof(0, ''));
		if (!this.stopped) this.relay.run();
	}

	private pointer = (e: PointerEvent) => {
		if (e.type === 'pointerdown' && e.button !== 0) return;
		const r = this.canvas.getBoundingClientRect();
		const k = this.canvas.width / Math.max(1, r.width);
		if (this.view.pointer((e.clientX - r.left) * k, (e.clientY - r.top) * k, e.type === 'pointermove' ? 0 : 1)) e.preventDefault();
		this.relay.run();
	};

	private key = (e: KeyboardEvent) => {
		if (e.ctrlKey || e.metaKey || e.altKey) return;
		if (this.view.key(e.key)) e.preventDefault();
		this.relay.run();
	};

	stop() {
		this.stopped = true;
		if (this.awake) this.o.onAwake?.(false);
		this.relay.close();
		removeEventListener('keydown', this.key);
		cancelAnimationFrame(this.raf);
		this.size.disconnect();
		this.canvas.remove();
		this.view.free();
	}
}
