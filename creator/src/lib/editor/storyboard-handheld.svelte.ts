// The handheld camera on the PC (#126; the studio's handheld.svelte.js – workflows/storyboard-studio README "Handheld
// camera", research/HANDHELD-CAMERA.md, STUDIO-IN-CREATOR §2.6): the pairing with the phone (a one-time code as a QR,
// the WebRTC offer / answer through the API, then the data channels peer to peer), the phone's pose mapped onto the shot
// camera (Recenter = the phone's pose now is the shot camera's), smoothed live and shown in the 3D view (Stage3d.hold),
// recording a take into the shot (inline, one undo step), a take's moment in the 3D view (the timeline), rendering a take
// into the workspace folder + its record. The maths and the takes are the tested storyboard-handheld / -takes modules.
import type { Shot, Take, TakeSample } from '@scutoons/content/storyboard';
import { api } from '$lib/api.ts';
import type { Stage3d } from '$lib/storyboard/stage.svelte.ts';
import { makeOrigin, mapPose, sampleAt, smooth, smoother, type Origin } from './storyboard-handheld.ts';
import { decodePhone, ICE_SERVERS, makeOffer, phoneUrl, send, sessionCalls, takeAnswer, waitAnswer, type Channels, type PhoneMode } from './storyboard-pairing.ts';
import { activeTake, addTake, newTake, renderRecord, sha256 } from './storyboard-takes.ts';
import { recordGeneration } from './storyboard-generations.ts';
import { nextGenerationFile } from './storyboard-workspace.ts';
import type { ShotEntry } from './storyboard-shots.ts';
import type { StoryboardPage } from './storyboard-page.svelte.ts';

export type LinkStatus = 'idle' | 'waiting' | 'connecting' | 'connected' | 'closed' | 'failed';

export class HandheldState {
	/** the pairing: its code and the phone's link (the QR) */
	code = $state<string | null>(null);
	url = $state<string | null>(null);
	status = $state<LinkStatus>('idle');
	error = $state<string | null>(null);
	/** what the phone says */
	phone = $state<{ ua: string; mode: PhoneMode; tracking: boolean } | null>(null);
	/** the phone's pose drives the 3D view */
	live = $state(false);
	/** live smoothing (new takes keep it) and scene metres per phone metre */
	smooth = $state(0);
	scale = $state(1);
	recording = $state(false);
	recSeconds = $state(0);
	/** a take is being rendered: "sb-1 · 12 / 48" */
	rendering = $state<string | null>(null);

	private stage: Stage3d | null = null;
	private pc: RTCPeerConnection | null = null;
	private ch: Channels | null = null;
	private stop: AbortController | null = null;
	private origin: Origin | null = null;
	private filter = smoother(0);
	private rec: { shot: string; t0: number | null; samples: TakeSample[] } | null = null;
	private readonly calls = sessionCalls(api);

	constructor(private readonly sb: StoryboardPage) {}

	// ── The 3D view ────────────────────────────────────────────────────────────────────────────────────────

	/** the 3D view's stage (shot-3d-view registers it while it is mounted) */
	attach(stage: Stage3d): void {
		this.stage = stage;
	}
	detach(stage: Stage3d): void {
		if (this.stage === stage) this.stage = null;
	}
	get held(): boolean {
		return !!this.stage?.held;
	}

	// ── Pairing ────────────────────────────────────────────────────────────────────────────────────────────

	/** a new code + QR; the PC's offer; then it waits for the phone (at most the code's 10 minutes) */
	async pair(): Promise<void> {
		this.unpair();
		const stop = (this.stop = new AbortController());
		this.status = 'waiting';
		this.error = null;
		try {
			const p = await this.calls.create({ toon: this.sb.toon, chapter: this.sb.chapter, shot: this.sb.current });
			if (stop.signal.aborted) return;
			this.code = p.code;
			this.url = phoneUrl(location.origin, p.code);
			const pc = (this.pc = new RTCPeerConnection({ iceServers: ICE_SERVERS }));
			pc.addEventListener('connectionstatechange', () => {
				if (this.pc !== pc) return;
				if (pc.connectionState === 'failed') this.status = 'failed';
				else if (pc.connectionState === 'disconnected' || pc.connectionState === 'closed') this.dropped();
			});
			const ch = await makeOffer(pc, this.calls, p.code);
			const answer = await waitAnswer(this.calls, p.code, { signal: stop.signal });
			if (!answer || stop.signal.aborted) return;
			this.status = 'connecting';
			this.ch = ch;
			ch.ctl.onmessage = (e) => this.onMessage(e.data);
			ch.pose.onmessage = (e) => this.onMessage(e.data);
			ch.ctl.onopen = () => {
				this.status = 'connected';
				this.sendState();
			};
			ch.ctl.onclose = () => this.dropped();
			await takeAnswer(pc, answer);
		} catch (e) {
			if (stop.signal.aborted) return;
			this.status = 'failed';
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/** Disconnect (or a new code): the link closes, an unused code goes */
	unpair(): void {
		this.stop?.abort();
		this.stop = null;
		if (this.code && this.status === 'waiting') void this.calls.remove(this.code).catch(() => undefined);
		this.stopLive();
		this.pc?.close();
		this.pc = null;
		this.ch = null;
		this.code = null;
		this.url = null;
		this.phone = null;
		this.status = 'idle';
	}

	private dropped(): void {
		if (this.status === 'idle') return;
		this.stopLive();
		this.phone = null;
		this.status = 'closed';
	}

	private onMessage(data: unknown): void {
		const m = decodePhone(data);
		if (!m) return;
		if (m.type === 'hello') this.phone = { ua: m.ua, mode: this.phone?.mode ?? '', tracking: true };
		else if (m.type === 'state') this.phone = { ua: this.phone?.ua ?? '', mode: m.mode, tracking: m.tracking };
		else if (m.type === 'rec') void (m.on ? this.startRec() : this.stopRec());
		else if (m.type === 'recenter') this.recenter();
		else if (m.type === 'pose') this.onPose(m.t, m.p, m.q);
	}

	/** the phone's view of the PC: the current shot, the camera on, recording */
	sendState(): void {
		const shot = this.sb.sel?.shot;
		send(this.ch?.ctl, { type: 'state', shot: this.sb.current, on: !!shot?.takes?.on, recording: this.recording, seconds: this.recSeconds });
	}

	// ── The live pose ──────────────────────────────────────────────────────────────────────────────────────

	private onPose(t: number, p: [number, number, number], q: [number, number, number, number]): void {
		const stage = this.stage;
		const rest = stage?.shotPose();
		if (!stage || !rest || !this.sb.sel?.shot.takes?.on) return;
		// the first pose (and the one after Recenter) starts at the shot camera
		this.origin ??= makeOrigin(p, q, rest.position, rest.quaternion);
		const pose = mapPose(this.origin, p, q, this.scale);
		const s: TakeSample = { t: t / 1000, position: pose.position, quaternion: pose.quaternion, fov: rest.fov };
		if (!this.live) {
			this.live = true;
			this.filter = smoother(this.smooth);
		}
		stage.hold(this.filter.step(s));
		if (this.rec) {
			this.rec.t0 ??= s.t;
			this.rec.samples.push({ ...s, t: s.t - this.rec.t0 });
			const sec = s.t - this.rec.t0;
			if (Math.floor(sec * 4) !== Math.floor(this.recSeconds * 4)) this.sendState();
			this.recSeconds = sec;
		}
	}
	/** the phone's pose now = the shot camera (on the next pose) */
	recenter(): void {
		this.origin = null;
	}
	setSmooth(v: number): void {
		this.smooth = v;
		this.filter = smoother(v);
	}
	setScale(v: number): void {
		this.scale = v;
		this.origin = null;
	}
	/** the phone's pose lets go of the 3D view (the shot camera back) */
	stopLive(): void {
		if (this.rec) void this.stopRec();
		this.live = false;
		this.origin = null;
		this.stage?.release();
	}

	// ── Recording ──────────────────────────────────────────────────────────────────────────────────────────

	startRec(): void {
		const id = this.sb.current;
		if (this.rec || !id || !this.sb.sel?.shot.takes?.on) return;
		this.rec = { shot: id, t0: null, samples: [] };
		this.recording = true;
		this.recSeconds = 0;
		this.sendState();
	}
	/** → the take (inline in the shot it was recorded for, active, one undo step) or null: nothing came */
	stopRec(): Take | null {
		const rec = this.rec;
		if (!rec) return null;
		this.rec = null;
		this.recording = false;
		this.sendState();
		const entry = this.sb.entries.find((e) => e.id === rec.shot);
		const take = entry && newTake(entry.shot.takes?.takes ?? [], rec.samples, { smooth: this.smooth, scale: this.scale });
		if (!entry || !take) return null;
		this.editShot(entry, addTake(take));
		return take;
	}

	/** a change of one shot by its place (the take's shot may not be the current one any more) */
	editShot(entry: ShotEntry, change: (shot: Shot) => void, key?: string): void {
		this.sb.doc.edit((file) => {
			const shot = entry.path.reduce((o: any, k) => o?.[k], file) as Shot | undefined;
			if (shot?.id === entry.id) change(shot);
		}, key);
	}

	// ── A take's moment in the 3D view (the timeline, the trim) ─────────────────────────────────────────────

	private cache = { key: '', samples: [] as TakeSample[] };
	/** the take smoothed with its own value (cached per take + value) */
	smoothed(take: Take): TakeSample[] {
		const key = `${take.id}/${take.smooth}/${take.samples.length}/${take.samples[0]?.t}`;
		if (this.cache.key !== key) this.cache = { key, samples: smooth(take.samples, take.smooth ?? 0) };
		return this.cache.samples;
	}
	/** the current shot's active take at `s` (s of the whole take) in the 3D view – false while it can't (a build) */
	preview(s: number): boolean {
		if (this.live) return true;
		const take = activeTake(this.sb.sel?.shot.takes);
		const sample = take && sampleAt(this.smoothed(take), s);
		return !!sample && !!this.stage?.hold(sample);
	}
	/** back to the shot camera ("Back to the shot camera", another clip) */
	endPreview(): void {
		if (!this.live) this.stage?.release();
	}

	// ── Rendering a take (the Sin City look → WebM → the workspace folder → its record) ───────────────────────

	async render(entry: ShotEntry, take: Take, fps = 24): Promise<void> {
		const sb = this.sb;
		if (sb.workspaceState !== 'ready') throw new Error('Choose the workspace folder first (the folder button above the view).');
		const [{ renderTake }, { shotSpec }, { personaModelUrls }, { base }] = await Promise.all([
			import('$lib/storyboard/render-take.ts'),
			import('@scutoons/content/storyboard'),
			import('$lib/storyboard/persona-models.ts'),
			import('$app/paths')
		]);
		const spec = shotSpec(entry.scene, $state.snapshot(entry.shot) as Shot, 1);
		this.rendering = `${entry.id} · 0 %`;
		try {
			const out = await renderTake(spec, $state.snapshot(take) as Take, { fps, models: personaModelUrls(base), progress: (n, all) => (this.rendering = `${entry.id} · ${Math.round((n / all) * 100)} %`) });
			const taken = [...sb.localFiles, ...sb.generations.filter((g) => g.shot === entry.id).map((g) => g.file ?? '')];
			const file = nextGenerationFile(entry.id, taken, 'webm');
			await sb.workspace.write(sb.toon, sb.chapter, file, out.blob);
			const hash = await sha256(out.blob);
			const item = await recordGeneration(api, renderRecord({ toon: sb.toon, chapter: sb.chapter, shot: entry.id, take, file, hash, machine: sb.machine, fps, seconds: out.seconds, ratio: entry.shot.aspect }));
			sb.upsert(item);
			await sb.refreshLocal();
		} finally {
			this.rendering = null;
		}
	}
}
