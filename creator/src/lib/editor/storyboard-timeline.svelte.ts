// The storyboard page's timeline along the bottom (#126; the studio's timeline.svelte.js – research/TIMELINE.md §1, §4,
// workflows/storyboard-studio README "The layout"): the PICKED shots' clips in the step's view mode (storyboard-
// timeline.ts), the one playhead, the transport (no Space key – the app owns Play: a video clip plays its <video> and is
// the clock, a take moves the 3D view's camera, a still waits 5 s), the trim (Take mode, way C: the shared Timeline's in /
// out / slip and I / O at the playhead write the take's in / out – one undo step per gesture). The main view follows the
// clip under the playhead: a take's moment in the 3D view, a video in the clip view (clip-view.svelte). In Shot / Take
// mode a seek's release and Play crossing into a clip make its shot the current one. Each step remembers its view mode
// and fold (localStorage `scutoons_creator:storyboard:timeline:<step>`, the browser's chrome).
import { untrack } from 'svelte';
import { api } from '$lib/api.ts';
import { browserStorage, STEPS } from './storyboard-steps.ts';
import { workFile } from './storyboard-generations.ts';
import { clipAt, clipsInOf, clipsLane, FRAME_MS, isVideo, kept, movedEdge, stepTimeline, trimAt, trimPoint, type ClipItem, type ClipsIn, type Edge, type Mode, type Over, type Word } from './storyboard-timeline.ts';
import { marksOf } from './storyboard-audition.ts';
import { setTrim } from './storyboard-takes.ts';
import type { StoryboardPage } from './storyboard-page.svelte.ts';

const storage = browserStorage();
const KEY = (step: string) => `scutoons_creator:storyboard:timeline:${step}`;
function read(step: string): unknown {
	try {
		return JSON.parse(storage?.getItem(KEY(step)) ?? 'null');
	} catch {
		return null;
	}
}

export class TimelineState {
	playhead = $state(0);
	zoom = $state(1);
	playing = $state(false);
	/** each step's view mode + fold */
	steps = $state<Record<string, { mode: Mode; folded: boolean }>>(Object.fromEntries(STEPS.map((s) => [s.id, stepTimeline(read(s.id), s.id)])));
	/** video lengths read from the files (generation id → ms) */
	known = $state<Record<number, number>>({});
	/** a video's object URL (generation id → blob: URL – the workspace folder's file or the work storage's) */
	urls = $state<Record<number, string>>({});
	/** a video that can't be read here (id → the reason) */
	missing = $state<Record<number, string>>({});
	/** the clip view's <video> while a video clip shows – the clock while it plays */
	video: HTMLVideoElement | null = null;
	/** the words of the sources (de / en, from the component) */
	words: Partial<Record<Word, string>> = {};

	constructor(private readonly sb: StoryboardPage) {}

	get here(): { mode: Mode; folded: boolean } {
		return this.steps[this.sb.step] ?? stepTimeline(null, this.sb.step);
	}
	private remember(next: { mode: Mode; folded: boolean }): void {
		this.steps[this.sb.step] = next;
		try {
			storage?.setItem(KEY(this.sb.step), JSON.stringify(next));
		} catch {
			// not remembered
		}
	}
	setMode(mode: Mode): void {
		this.stop();
		this.remember({ ...this.here, mode });
	}
	setFolded(folded: boolean): void {
		if (folded !== this.here.folded) this.remember({ ...this.here, folded });
		if (folded) this.stop();
	}

	/** each picked shot's clips (from the shot and the chapter's records) */
	readonly clips = $derived.by<Record<string, ClipsIn>>(() => {
		const out: Record<string, ClipsIn> = {};
		for (const e of this.sb.entries) if (this.sb.picked.includes(e.id)) out[e.id] = clipsInOf(e.shot, this.sb.generations);
		return out;
	});
	/** the lane in the step's mode: { mode, lane, items, span, probe, sel (the current shot's clip) } */
	readonly view = $derived.by(() => {
		const mode = this.here.mode;
		const ids = this.sb.order.filter((id) => this.sb.picked.includes(id));
		// the Variants board's current card auditions as the current shot's Generated clip (#125); its marks = the bullets
		const a = this.sb.board.audition;
		const g = a?.video && this.sb.current ? this.sb.generations.find((x) => x.id === a.id) : null;
		const over: Record<string, Over> = g ? { [g.shot]: { pick: { id: g.id, file: g.file, ms: g.duration && g.duration > 0 ? Math.round(g.duration * 1000) : null, approved: g.approved } } } : {};
		const { lane, span, probe } = clipsLane(ids, mode, this.clips, this.known, this.words, 'Clips', over, marksOf(this.sb.generations));
		return { mode, lane, items: lane.items, span, probe, sel: lane.items.find((i) => i.id === this.sb.current) ?? null };
	});
	/** the clip under the playhead */
	readonly under = $derived(clipAt(this.view.items, this.playhead));

	// ── The media ──────────────────────────────────────────────────────────────────────────────────────────

	/** a generation's video as an object URL (once per id): the workspace folder first, else the work storage */
	async media(id: number): Promise<string | null> {
		if (this.urls[id]) return this.urls[id];
		const g = this.sb.generations.find((x) => x.id === id);
		if (!g) return null;
		try {
			const blob = g.file && this.sb.localFiles.has(g.file) ? await this.sb.readLocal(g.file) : g.work_url ? await workFile(api, g.work_url) : null;
			if (!blob) {
				this.missing = { ...this.missing, [id]: g.machine ? `on ${g.machine}` : 'not here' };
				return null;
			}
			const url = URL.createObjectURL(blob);
			this.urls = { ...this.urls, [id]: url };
			return url;
		} catch (e) {
			this.missing = { ...this.missing, [id]: e instanceof Error ? e.message : String(e) };
			return null;
		}
	}
	/** a video's length from its metadata (the record had none) */
	async probe(id: number): Promise<void> {
		const url = await this.media(id);
		if (!url || this.known[id]) return;
		const v = document.createElement('video');
		v.preload = 'metadata';
		v.muted = true;
		await new Promise<void>((done) => {
			v.onloadedmetadata = () => {
				if (Number.isFinite(v.duration) && v.duration > 0) this.known = { ...this.known, [id]: Math.round(v.duration * 1000) };
				done();
			};
			v.onerror = () => done();
			v.src = url;
		});
		v.removeAttribute('src');
		v.load();
	}
	dispose(): void {
		this.stop();
		Object.values(this.urls).forEach((u) => URL.revokeObjectURL(u));
	}

	// ── What the Timeline asks ─────────────────────────────────────────────────────────────────────────────

	private follows = () => this.view.mode === 'shot' || this.view.mode === 'take';
	seek(t: number, final: boolean): void {
		this.stop();
		this.playhead = t;
		if (!final || !this.follows()) return;
		const it = clipAt(this.view.items, t)?.item;
		if (it && it.id !== this.sb.current) this.sb.current = it.id;
	}
	/** a click on a clip: its shot becomes the current one (the panels edit it) */
	select(id: string): void {
		this.stop();
		if (id !== this.sb.current) this.sb.current = id;
	}
	/** the trim (Take mode): `k` = the kept part (ms of the take); a gesture = calls with final false, then one true –
	 *  one undo step; the playhead on the moved point (the 3D view shows the take's camera there) */
	trim(id: string, k: { in: number; out: number }, final: boolean, edge?: Edge): void {
		this.stop();
		const it = this.view.items.find((i) => i.id === id);
		const entry = this.sb.entries.find((e) => e.id === id);
		const take = it?.clip.take;
		if (!it?.trim || !entry || !take) return;
		const e = edge ?? movedEdge(it.trim, k);
		this.playhead = trimPoint(it, k, e, this.view.lane.step ?? FRAME_MS);
		if (id !== this.sb.current) this.sb.current = id;
		this.sb.hh.editShot(entry, setTrim(take.id, k.in, k.out), `${id}:trim`);
		void final;
	}
	/** I / O: the take under the playhead trimmed at the playhead (Take mode); false = nothing there to trim */
	trimAtPlayhead(edge: 'in' | 'out'): boolean {
		if (this.view.mode !== 'take') return false;
		const at = trimAt(this.view.items, this.playhead, edge);
		if (!at) return false;
		this.trim(at.item.id, { in: at.in, out: at.out }, true, edge);
		return true;
	}

	// ── The main view follows the playhead ─────────────────────────────────────────────────────────────────

	/** the page: the playhead to the current shot's clip when it changes; a take's moment into the 3D view */
	follow(): void {
		const key = $derived(`${this.sb.current}|${this.view.mode}|${this.view.items.map((i) => i.id)}|${this.view.sel?.start}|${this.view.sel?.length}`);
		$effect(() => {
			void key;
			untrack(() => {
				const s = this.view.sel;
				if (s && !this.playing && !(this.playhead >= s.start && this.playhead < s.start + s.length)) this.playhead = s.start + (s.trim?.in ?? 0);
			});
		});
		// a take of the current shot under the playhead: its camera at that moment; anything else: the shot camera back
		const moment = $derived.by(() => {
			const c = this.under;
			const it = c?.item;
			return it && it.clip.source === 'take' && it.id === this.sb.current && it.clip.take ? (it.clip.from + c.off) / 1000 : null;
		});
		$effect(() => {
			const at = moment;
			const shown = this.here.folded;
			untrack(() => {
				if (at !== null && !shown) this.sb.hh.preview(at);
				else this.sb.hh.endPreview();
			});
		});
		$effect(() => {
			for (const id of this.view.probe) void this.probe(id);
		});
		$effect(() => () => this.dispose());
	}

	// ── The transport ──────────────────────────────────────────────────────────────────────────────────────

	private raf = 0;
	private last = 0;
	toggle(): void {
		if (this.playing) this.stop();
		else this.start();
	}
	private start(): void {
		const v = this.view;
		if (!v.items.length) return;
		this.playhead = kept(v.items, this.playhead >= v.span - 1 ? 0 : this.playhead);
		this.playing = true;
		this.last = performance.now();
		this.raf = requestAnimationFrame(this.tick);
	}
	stop(): void {
		if (!this.playing) return;
		this.playing = false;
		cancelAnimationFrame(this.raf);
	}
	/** the clock: a video clip's own time while it plays, else the frame time */
	private clockAt(it: ClipItem, t: number, dt: number): number {
		const el = this.video;
		if (!isVideo(it.clip.source) || !el || Number(el.dataset.gen) !== it.clip.gen || el.error) return t + dt;
		if (el.ended) return it.start + it.length;
		if (!el.paused || el.readyState < 2) return it.start + Math.min(el.currentTime * 1000 - it.clip.from, it.length);
		return t + dt;
	}
	private readonly tick = (now: number) => {
		if (!this.playing) return;
		const dt = now - this.last;
		this.last = now;
		this.raf = requestAnimationFrame(this.tick);
		const v = this.view;
		const c = clipAt(v.items, this.playhead);
		if (!c) return this.stop();
		const t = kept(v.items, this.clockAt(c.item, this.playhead, dt));
		if (t >= v.span) {
			this.playhead = v.span;
			return this.stop();
		}
		this.playhead = t;
		const it = clipAt(v.items, t)?.item;
		if (it && this.follows() && it.id !== this.sb.current) this.sb.current = it.id;
	};
}
