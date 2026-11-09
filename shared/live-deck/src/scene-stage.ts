// scene-stage.ts (#463): a scene playing in an element – the renderer every deck draws with, compiled to wasm
// (deck/space-web `SceneView`). Only the canvas, its size and the animation frames are here.
import { spaceModule, stageCanvas } from './space.ts';

/** #463: the wasm-bindgen API of deck/space-web's `SceneView` – a scene drawn by the renderer every deck uses */
export interface SceneView {
	backend(): string;
	resize(w: number, h: number): void;
	frame(dt: number, time: number): void;
	free(): void;
}

export interface SceneOptions {
	host: HTMLElement;
	/** the URL of the folder with space_web.js and space_web_bg.wasm (ends with /) */
	base: string;
	/** a scene document; none = the core's default, the idle scene */
	scene?: string;
	prefer?: 'auto' | 'webgl2';
	maxScale?: number;
}

/** #463: a scene playing in `host` – the wasm draws it; here only the canvas, its size and the animation frames */
export class SceneStage {
	private raf = 0;
	private last = -1;
	private readonly size: ResizeObserver;

	private constructor(
		private readonly o: SceneOptions,
		private readonly canvas: HTMLCanvasElement,
		private readonly view: SceneView
	) {
		this.size = new ResizeObserver(() => this.fit());
		this.size.observe(o.host);
		this.fit();
		this.raf = requestAnimationFrame(this.frame);
	}

	/** Load the wasm, make the view (WebGPU, else WebGL2 on a fresh canvas), start drawing. */
	static async start(o: SceneOptions): Promise<SceneStage> {
		const mod = await spaceModule(o.base);
		let canvas = stageCanvas(o);
		try {
			return new SceneStage(o, canvas, await mod.SceneView.create(canvas, o.scene ?? '', o.prefer ?? 'auto'));
		} catch (e) {
			if (o.prefer === 'webgl2') throw e;
			console.warn('scene: WebGPU failed, WebGL2 instead', e);
			canvas.remove();
			canvas = stageCanvas(o);
			return new SceneStage(o, canvas, await mod.SceneView.create(canvas, o.scene ?? '', 'webgl2'));
		}
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
	};

	stop() {
		cancelAnimationFrame(this.raf);
		this.size.disconnect();
		this.canvas.remove();
		this.view.free();
	}
}
