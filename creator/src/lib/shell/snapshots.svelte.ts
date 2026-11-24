// Workspace snapshots (#842): what the front workspace looks like, taken once it held still (stills.ts `snapshotDue`)
// and kept in memory; leaving it – another workspace, the dashboard, the window hidden – sends the last one up as the
// member's own `workspace:<id>` still. The picture: the stage's canvas where one fills it (a 3D place, a scene), else a
// DOM capture of the window without the overlays (modern-screenshot, loaded on first use).
import { workspaceThumbKey } from '@scutoons/api';
import { shared } from '$lib/utils/shared.ts';
import { snapshotDue } from './stills.ts';
import { thumbs, toStill } from './thumbs.svelte.ts';

/** what never belongs in a workspace's picture: the overlays over it */
const SKIP = '[data-testid="dashboard"], [data-testid="launcher"], [data-testid="user-menu"], [data-snapshot-skip]';
/** a canvas counts as the stage's picture when it covers this share of the stage */
const CANVAS_SHARE = 0.4;
/** workspaces without a picture of their own */
const NONE = new Set(['home']);

const frame = () => new Promise<void>((done) => requestAnimationFrame(() => done()));

/** the stage's canvas drawn now (right after a frame, while a WebGL buffer still holds it); null when blank or none */
async function stageCanvas(): Promise<HTMLCanvasElement | null> {
	const stage = document.querySelector<HTMLElement>('[data-testid="stage"]');
	if (!stage) return null;
	const area = stage.clientWidth * stage.clientHeight;
	const big = [...stage.querySelectorAll('canvas')]
		.map((c) => ({ c, r: c.getBoundingClientRect() }))
		.filter(({ c, r }) => c.width > 0 && c.height > 0 && r.width * r.height >= area * CANVAS_SHARE && !c.closest(SKIP))
		.sort((a, b) => b.r.width * b.r.height - a.r.width * a.r.height)[0]?.c;
	if (!big) return null;
	await frame();
	const copy = document.createElement('canvas');
	copy.width = big.width;
	copy.height = big.height;
	const ctx = copy.getContext('2d', { willReadFrequently: true });
	if (!ctx) return null;
	ctx.drawImage(big, 0, 0);
	const step = Math.max(1, Math.floor((copy.width * copy.height) / 400));
	const px = ctx.getImageData(0, 0, copy.width, copy.height).data;
	for (let i = 3; i < px.length; i += 4 * step) if (px[i] !== 0) return copy;
	return null;
}

/** the window as it shows the workspace, without the overlays, small */
async function domCapture(): Promise<HTMLCanvasElement | null> {
	const root = document.querySelector<HTMLElement>('[data-testid="creator"]');
	if (!root) return null;
	const { domToCanvas } = await import('modern-screenshot');
	return domToCanvas(root, {
		scale: Math.min(1, 640 / Math.max(1, root.clientWidth)),
		backgroundColor: getComputedStyle(document.body).backgroundColor,
		filter: (node) => !(node instanceof Element && node.matches(SKIP)),
		font: false,
		timeout: 4000
	});
}

/** the front workspace's picture now, as a still */
export async function captureWorkspace(): Promise<Blob | null> {
	try {
		const canvas = (await stageCanvas()) ?? (await domCapture());
		return canvas ? await toStill(canvas, canvas.width, canvas.height) : null;
	} catch {
		return null;
	}
}

class Snapshots {
	/** this visit's newest picture per workspace (an object URL) – the dashboard shows it before the server has it */
	local = $state.raw<Record<string, string>>({});
	#held = new Map<string, Blob>();
	#taken = new Map<string, number>();
	#front: string | null = null;
	#away = false;
	#lastInput = Date.now();
	#timer: ReturnType<typeof setTimeout> | null = null;
	#running = false;
	#taking: Promise<void> | null = null;

	start() {
		if (this.#running || typeof window === 'undefined') return;
		this.#running = true;
		for (const type of ['pointerdown', 'keydown', 'wheel']) addEventListener(type, this.#input, { capture: true, passive: true });
		document.addEventListener('visibilitychange', this.#onHidden);
		addEventListener('pagehide', this.#sendAll);
		this.#schedule();
	}

	stop() {
		if (!this.#running) return;
		this.#running = false;
		for (const type of ['pointerdown', 'keydown', 'wheel']) removeEventListener(type, this.#input, { capture: true });
		document.removeEventListener('visibilitychange', this.#onHidden);
		removeEventListener('pagehide', this.#sendAll);
		if (this.#timer) clearTimeout(this.#timer);
		this.#timer = null;
	}

	/**
	 * The front workspace and whether something lies over it (the dashboard): a workspace left sends its last picture; the
	 * dashboard coming up takes a fresh one first (the workspace still shows under it).
	 */
	front(id: string, away: boolean) {
		const was = this.#front;
		const wasAway = this.#away;
		this.#front = id;
		this.#away = away;
		if (was !== id) this.#lastInput = Date.now();
		if (!this.#running) return;
		if (was && was !== id) void this.#send(was);
		if (was === id && away && !wasAway) void this.#take(id).then(() => this.#send(id));
		if (!away) this.#schedule();
	}

	#input = () => {
		this.#lastInput = Date.now();
		this.#schedule();
	};

	#onHidden = () => {
		if (document.visibilityState !== 'hidden') return;
		const id = this.#front;
		if (id && !this.#away) void this.#take(id).then(() => this.#send(id));
		else this.#sendAll();
	};

	#sendAll = () => {
		for (const id of [...this.#held.keys()]) void this.#send(id);
	};

	#schedule() {
		if (this.#timer) clearTimeout(this.#timer);
		this.#timer = null;
		const id = this.#front;
		if (!this.#running || !id || this.#away || NONE.has(id)) return;
		const wait = Math.max(0, snapshotDue(this.#lastInput, this.#taken.get(id) ?? null) - Date.now());
		this.#timer = setTimeout(() => {
			this.#timer = null;
			if (this.#front === id && !this.#away && document.visibilityState === 'visible') void this.#take(id);
		}, wait);
	}

	async #take(id: string): Promise<void> {
		if (NONE.has(id)) return;
		if (this.#taking) await this.#taking;
		this.#taking = (async () => {
			const still = await captureWorkspace();
			this.#taken.set(id, Date.now());
			if (!still) return;
			this.#held.set(id, still);
			const was = this.local[id];
			this.local = { ...this.local, [id]: URL.createObjectURL(still) };
			if (was) URL.revokeObjectURL(was);
		})();
		try {
			await this.#taking;
		} finally {
			this.#taking = null;
		}
	}

	async #send(id: string): Promise<void> {
		const still = this.#held.get(id);
		if (!still) return;
		this.#held.delete(id);
		await thumbs.put(workspaceThumbKey(id), still, new Date().toISOString());
	}
}

export const snapshots = shared('creator-snapshots', () => new Snapshots());
