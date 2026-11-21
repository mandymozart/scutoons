// The XR stage's page side (#482, agents/research/XR.md §10.8): the shapes the workspace and the engine's stage entry
// share, and what the stage's panel shows. The stage runs in the creator's own page (owner 2026-11-21: no sandbox – the
// member is trusted, as with the API); pure.

export const STAGE_VIEWS = ['reader', 'top', 'low', 'far'] as const;
export type StageView = (typeof STAGE_VIEWS)[number];
export const STAGE_TOOLS = ['select', 'translate', 'rotate', 'scale'] as const;
export type StageTool = (typeof STAGE_TOOLS)[number];
/** where the creator serves the stage's own files (the desk, its textures, the sky) */
export const STAGE_FILES_PATH = '/xr-stage/';

const STATUSES = ['idle', 'loading', 'ready', 'starting', 'running', 'paused', 'error'] as const;
export type StageStatus = (typeof STATUSES)[number];

/** a node of a target's entity, or the target's printed image (`path` null) – xr-nodes.ts `NodeRef` */
export interface StageRef {
	target: string;
	path: number[] | null;
}

/** a node's placement as the configuration stores it */
export interface StagePlacement {
	position: [number, number, number];
	rotation: [number, number, number];
	scale: number | [number, number, number];
}

export interface StagePlugin {
	id: string;
	api: string;
	code: string;
}

/** an experience's document as what the stage opens: its configuration and the plugins that are whole; null without one */
export function stageInput(content: unknown): { configuration: object; plugins: StagePlugin[] } | null {
	const doc = content && typeof content === 'object' ? (content as Record<string, unknown>) : {};
	const configuration = doc.configuration;
	if (!configuration || typeof configuration !== 'object') return null;
	const plugins = (Array.isArray(doc.plugins) ? doc.plugins : []).flatMap((p: unknown): StagePlugin[] => {
		const plugin = p && typeof p === 'object' ? (p as Record<string, unknown>) : {};
		return typeof plugin.id === 'string' && typeof plugin.api === 'string' && typeof plugin.code === 'string' ? [{ id: plugin.id, api: plugin.api, code: plugin.code }] : [];
	});
	return { configuration, plugins };
}

export interface StageShown {
	phase: 'loading' | 'playing' | 'failed';
	/** why it failed: the configuration's problems, or the engine's message */
	problems: string[];
	subtitle: string | null;
}
export const STAGE_LOADING: StageShown = { phase: 'loading', problems: [], subtitle: null };

/** what the stage's panel shows after the engine's status */
export function stageAfter(shown: StageShown, status: StageStatus, message: string | null): StageShown {
	if (status === 'error') return { ...shown, phase: 'failed', problems: message ? [message] : [] };
	if (status === 'running') return { ...shown, phase: 'playing', problems: [] };
	return shown;
}
