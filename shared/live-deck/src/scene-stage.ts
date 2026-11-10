// scene-stage.ts (#463, #465): a scene playing in an element – the renderer every deck draws with, compiled to wasm
// (deck/space-web `SceneView`). Here only the canvas, its size, the animation frames and the fetches the core asks for.
import { sniff, spaceModule, stageCanvas } from './space.ts';
import { askPortraits, fetchChapter, fetchSet, fetchSlice, portraitsAt, type ChapterView, type SetView } from './stage-assets.ts';

/** #463: the wasm-bindgen API of deck/space-web's `SceneView` */
export interface SceneView extends SetView, ChapterView {
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
}

/** #463: a scene playing in `host` */
export class SceneStage {
	stopped = false;
	private raf = 0;
	private last = -1;
	private readonly size: ResizeObserver;
	private readonly portraitsByChar = new Map<string, Promise<Record<string, { path: string }>>>();

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
		askPortraits(this, this.o.cdn, (c) => portraitsAt(this.o.api, this.portraitsByChar, c));
		const asked = this.view.slice_requests();
		for (let i = 0; i + 1 < asked.length; i += 2) void fetchSlice(this, this.o.cdn + (asked[i + 1] as string), asked[i] as number);
	};

	stop() {
		this.stopped = true;
		cancelAnimationFrame(this.raf);
		this.size.disconnect();
		this.canvas.remove();
		this.view.free();
	}
}
