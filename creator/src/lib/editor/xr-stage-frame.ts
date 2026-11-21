// The XR stage's frame (#482, agents/research/XR.md §10.9): an experience's plugins are unchecked code, so the stage
// that runs them is a document of its own, loaded into an iframe sandboxed without `allow-same-origin` – an opaque
// origin: no session, no storage, no way into the creator's page (OSCT's editor does the same). The page and the frame
// talk only through postMessage; this file is both sides' shapes and checks, and the frame's content policy.

export const STAGE_FRAME_PATH = '/xr-stage-frame';
/** scripts and nothing else: no `allow-same-origin`, no forms, popups or navigation of the page */
export const STAGE_SANDBOX = 'allow-scripts';
/** where the creator serves the stage's own files (the desk, its textures, the sky) */
export const STAGE_FILES_PATH = '/xr-stage/';
const TAG = 'scutoons-xr-stage';

export const STAGE_VIEWS = ['reader', 'top', 'low', 'far'] as const;
export type StageView = (typeof STAGE_VIEWS)[number];
const STATUSES = ['idle', 'loading', 'ready', 'starting', 'running', 'paused', 'error'] as const;
export type StageStatus = (typeof STATUSES)[number];

export interface StagePlugin {
	id: string;
	api: string;
	code: string;
}

/** page → frame */
export type StageRequest =
	| { kind: 'open'; configuration: object; plugins: StagePlugin[]; scene: string; assets: string; language: string }
	| { kind: 'view'; view: StageView };

/** frame → page */
export type StageEvent =
	| { kind: 'ready' }
	| { kind: 'status'; status: StageStatus; message: string | null }
	| { kind: 'problems'; problems: string[] }
	| { kind: 'found'; targets: string[] }
	| { kind: 'subtitle'; text: string | null };

type Obj = Record<string, unknown>;
const tagged = (data: unknown): Obj | null => (!!data && typeof data === 'object' && (data as Obj).tag === TAG ? (data as Obj) : null);
const text = (v: unknown, max: number): string | null => (typeof v === 'string' ? v.slice(0, max) : null);
const texts = (v: unknown, count: number, max: number): string[] | null =>
	Array.isArray(v) && v.every((x) => typeof x === 'string') ? v.slice(0, count).map((x: string) => x.slice(0, max)) : null;

export const stageMessage = (message: StageRequest | StageEvent): object => ({ tag: TAG, ...message });

/** an experience's document as the request that opens it; null when it holds no configuration */
export function openRequest(content: unknown, scene: string, assets: string, language: string): StageRequest | null {
	const doc = content && typeof content === 'object' ? (content as Obj) : {};
	const configuration = doc.configuration;
	if (!configuration || typeof configuration !== 'object' || !scene) return null;
	const plugins = (Array.isArray(doc.plugins) ? doc.plugins : []).flatMap((p: unknown): StagePlugin[] => {
		const plugin = p && typeof p === 'object' ? (p as Obj) : {};
		return typeof plugin.id === 'string' && typeof plugin.api === 'string' && typeof plugin.code === 'string' ? [{ id: plugin.id, api: plugin.api, code: plugin.code }] : [];
	});
	return { kind: 'open', configuration, plugins, scene, assets, language };
}

/** what the page sent, in the frame; anything else: null */
export function readStageRequest(data: unknown): StageRequest | null {
	const d = tagged(data);
	if (!d) return null;
	if (d.kind === 'view') return STAGE_VIEWS.includes(d.view as StageView) ? { kind: 'view', view: d.view as StageView } : null;
	if (d.kind !== 'open') return null;
	const request = openRequest({ configuration: d.configuration, plugins: d.plugins }, String(d.scene ?? ''), String(d.assets ?? ''), String(d.language ?? ''));
	return request && /^https?:\/\//.test(String(d.assets)) ? request : null;
}

/**
 * What the frame sent, in the page. The frame runs plugin code, which may post anything: only these shapes pass, cut
 * to sizes a status line can show – and none of them does more than change what the stage's panel shows.
 */
export function readStageEvent(data: unknown): StageEvent | null {
	const d = tagged(data);
	if (!d) return null;
	if (d.kind === 'ready') return { kind: 'ready' };
	if (d.kind === 'status') return STATUSES.includes(d.status as StageStatus) ? { kind: 'status', status: d.status as StageStatus, message: text(d.message, 300) } : null;
	if (d.kind === 'problems') {
		const problems = texts(d.problems, 20, 300);
		return problems ? { kind: 'problems', problems } : null;
	}
	if (d.kind === 'found') {
		const targets = texts(d.targets, 200, 100);
		return targets ? { kind: 'found', targets } : null;
	}
	if (d.kind === 'subtitle') return d.text === null || typeof d.text === 'string' ? { kind: 'subtitle', text: text(d.text, 500) } : null;
	return null;
}

export interface StageShown {
	phase: 'loading' | 'playing' | 'failed';
	/** why it failed: the configuration's problems, or the engine's message */
	problems: string[];
	found: string[];
	subtitle: string | null;
}
export const STAGE_LOADING: StageShown = { phase: 'loading', problems: [], found: [], subtitle: null };

/** what the stage's panel shows after an event of the frame */
export function stageAfter(shown: StageShown, event: StageEvent): StageShown {
	if (event.kind === 'problems') return { ...shown, phase: 'failed', problems: event.problems };
	if (event.kind === 'found') return { ...shown, found: event.targets };
	if (event.kind === 'subtitle') return { ...shown, subtitle: event.text };
	if (event.kind !== 'status') return shown;
	if (event.status === 'error') return { ...shown, phase: 'failed', problems: event.message ? [event.message] : [] };
	if (event.status === 'running') return { ...shown, phase: 'playing', problems: [] };
	return shown;
}

/**
 * The frame's content policy, set before the experience and its plugins load: scripts from the creator's own files
 * and the plugins' blobs, files from the creator and the experience's folder, nothing else – no API, no other host.
 * `app`: the creator's origin; `assets`: the experience's folder; `devSocket`: the dev server's reload socket.
 */
export function stageCsp({ app, assets, devSocket = null }: { app: string; assets: string; devSocket?: string | null }): string {
	const folder = `${assets.replace(/\/+$/, '')}/`;
	const files = `${app} ${folder} blob: data:`;
	return [
		"default-src 'none'",
		`script-src ${app} blob: 'wasm-unsafe-eval'`,
		`style-src ${app} 'unsafe-inline'`,
		`connect-src ${files}${devSocket ? ` ${devSocket}` : ''}`,
		`img-src ${files}`,
		`media-src ${files}`,
		`font-src ${app} data:`,
		"worker-src 'none'",
		"frame-src 'none'",
		"object-src 'none'",
		"base-uri 'none'",
		"form-action 'none'"
	].join('; ');
}
