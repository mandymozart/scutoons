// scene-stage.ts (#463, #465): a scene playing in an element – the renderer every deck draws with, compiled to wasm
// (deck/space-web `SceneView`). Here only the canvas, its size, the animation frames and the fetches the core asks for.
import { sniff, spaceModule, stageCanvas, type SpaceModule } from './space.ts';
import { askPortraits, fetchChapter, fetchOutletAsks, fetchSet, fetchSlice, portraitsAt, type ChapterView, type OutletView, type SetView } from './stage-assets.ts';
import { RelaySocket, type RelayCoreView } from './client.ts';
import { driveLink, ensureLink, type Paired } from './deck-key.ts';

/** #463: the wasm-bindgen API of deck/space-web's `SceneView` */
export interface SceneView extends SetView, ChapterView, OutletView {
	join_outlet(relay: string, outlet: string, locale: string, api: string): boolean;
	deck_link(why: string, code: string, app: string): void;
	set_request(): string;
	reloaded(): number;
	resize(w: number, h: number): void;
	frame(dt: number, time: number): void;
	room(): string;
	chapters(): string[];
	slice_requests(): unknown[];
	free(): void;
}

export interface SceneOptions {
	host: HTMLElement;
	/** the URL of the folder with space_web.js and space_web_bg.wasm (ends with /) */
	base: string;
	/** the library API and the CDN (ends with /) */
	api: string;
	cdn: string;
	/** a built-in scene's name or a scene document; none = the core's default, the idle scene */
	scene?: string;
	prefer?: 'auto' | 'webgl2';
	maxScale?: number;
	/** #465: the Live outlet an `outlet` node shows (relay "" = the hosted show) */
	outlet?: { relay: string; name: string; locale: string };
	/** #470: the build label the deck names when it asks to be paired */
	build?: string;
	/** #484: the viewer's sign-in, if any – a signed-in browser deck is its user and is not paired */
	token?: () => string | null;
}

/** #463: a scene playing in `host` */
export class SceneStage {
	stopped = false;
	private raf = 0;
	private last = -1;
	private readonly size: ResizeObserver;
	private readonly portraitsByChar = new Map<string, Promise<Record<string, { path: string }>>>();
	/** #465: the outlet's relay socket (the client is the view's) */
	private relay: RelaySocket | null = null;
	private spaceModule: SpaceModule | null = null;
	/** #470: the deck's pairing (an outlet's show media need its token) and the key it signs with – #484: made only
	 * when nobody is signed in (deck-key.ts `ensureLink`) */
	link: Paired | null = null;
	pairing = false;

	private constructor(
		private readonly o: SceneOptions,
		private readonly canvas: HTMLCanvasElement,
		readonly view: SceneView
	) {
		this.size = new ResizeObserver(() => this.fit());
		this.size.observe(o.host);
		this.fit();
		this.raf = requestAnimationFrame(this.frame);
	}

	/** Load the wasm, make the view (WebGPU, else WebGL2 on a fresh canvas), start drawing and fetching. */
	static async start(o: SceneOptions): Promise<SceneStage> {
		const mod = await spaceModule(o.base);
		let canvas = stageCanvas(o);
		let view: SceneView;
		try {
			view = await mod.SceneView.create(canvas, o.scene ?? '', o.prefer ?? 'auto');
		} catch (e) {
			if (o.prefer === 'webgl2') throw e;
			console.warn('scene: WebGPU failed, WebGL2 instead', e);
			canvas.remove();
			canvas = stageCanvas(o);
			view = await mod.SceneView.create(canvas, o.scene ?? '', 'webgl2');
		}
		const s = new SceneStage(o, canvas, view);
		s.spaceModule = mod;
		if (o.outlet && view.join_outlet(o.outlet.relay, o.outlet.name, o.outlet.locale, o.api)) {
			(s.relay = new RelaySocket(view as unknown as RelayCoreView, {})).start();
		}
		const room = view.room();
		if (room) void fetchSet(s, mod.SpaceView.encoded_images, sniff, o, room).catch((e) => console.warn('scene: the set failed', e));
		view.chapters().forEach((path, i) => void fetchChapter(s, o.api + path, i));
		return s;
	}

	/** "webgpu" or "webgl2" */
	get backend(): string {
		return this.view.backend();
	}

	private fit() {
		const k = Math.min(devicePixelRatio || 1, this.o.maxScale ?? 2);
		this.view.resize(this.canvas.clientWidth * k, this.canvas.clientHeight * k);
	}

	private frame = (ms: number) => {
		this.raf = requestAnimationFrame(this.frame);
		const now = ms / 1000;
		this.view.frame(this.last < 0 ? 0 : now - this.last, now);
		this.last = now;
		const at = this.view.reloaded();
		if (at >= 0) this.view.chapters().forEach((path, i) => void fetchChapter(this, this.o.api + path, at + i));
		askPortraits(this, this.o.cdn, (c) => portraitsAt(this.o.api, this.portraitsByChar, c));
		const asked = this.view.slice_requests();
		for (let i = 0; i + 1 < asked.length; i += 2) void fetchSlice(this, this.o.cdn + (asked[i + 1] as string), asked[i] as number);
		if (!this.relay) return;
		// what the outlet says (media.ready) goes out; what it asks for is fetched; a cell's set loads
		this.relay.run();
		// #484: signed in – the deck is its user; else its own key and the pairing (a sign-out mid-show lands here)
		const user = this.o.token?.() ?? undefined;
		if (!user && this.spaceModule) void ensureLink(this, this.spaceModule.DeckLink, this.o.build ?? '');
		const link = user ? null : this.link;
		if (link) void driveLink(link, this.o.api);
		this.view.deck_link(user ? '' : link ? link.view.why(ms) : 'this browser cannot pair (no WebCrypto Ed25519)', link?.view.code() ?? '', location.origin);
		fetchOutletAsks(this, { api: this.o.api, cdn: this.o.cdn, link: link?.view, user });
		const set = this.view.set_request();
		if (set && this.spaceModule) void fetchSet(this, this.spaceModule.SpaceView.encoded_images, sniff, this.o, set).catch((e) => console.warn('scene: the set failed', e));
	};

	stop() {
		this.stopped = true;
		this.relay?.close();
		this.link?.view.free();
		cancelAnimationFrame(this.raf);
		this.size.disconnect();
		this.canvas.remove();
		this.view.free();
	}
}
