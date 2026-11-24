// This machine's settings of the creator (research/redesign/PLAN.md §2.5, the deck's settings.rs): the look, motion,
// the layout – kept in this browser. Member settings (language, AI, notifications) live with the member. Pure:
// test/shell.test.ts.
import { clampSplit } from './stage.ts';

/** where this browser keeps them */
export const SETTINGS_KEY = 'sc-creator-settings';

export type Theme = 'night' | 'day';

export interface MachineSettings {
	theme: Theme;
	/** ok, warning and error in the colour-blind set (`data-colorblind`, kit/tokens.toml [day|night.colorblind]) */
	colorblind: boolean;
	/** the UI size: 1rem as a share of the default (0.8 … 1.5) */
	scale: number;
	panelTransitions: boolean;
	reduceMotion: boolean;
	/** the last tabs come back when the creator starts */
	reopenTabs: boolean;
	/** a 3D stage's gizmo handles show on the selection even with the select tool (#482); off: only while moving, turning or scaling */
	gizmoAlways: boolean;
	/** per workspace: where a split of its own view divides it, and whether its content keeps clear of the bar's row */
	views: Record<string, ViewSettings>;
}

export interface ViewSettings {
	/** where the workspace's own view splits (a share of the width; the character's views) */
	split?: number;
	/** the content starts under the bar's row (the stage itself still runs under the bar); default on */
	clear?: boolean;
}

export const DEFAULTS: MachineSettings = { theme: 'day', colorblind: false, scale: 1, panelTransitions: true, reduceMotion: false, reopenTabs: true, gizmoAlways: false, views: {} };

/** a workspace's view settings, every field checked */
export function readViews(raw: unknown): Record<string, ViewSettings> {
	if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return {};
	const out: Record<string, ViewSettings> = {};
	for (const [kind, v] of Object.entries(raw as Record<string, unknown>)) {
		if (typeof v !== 'object' || v === null) continue;
		const r = v as Record<string, unknown>;
		const view: ViewSettings = {};
		if (typeof r.split === 'number') view.split = clampSplit(r.split);
		if (typeof r.clear === 'boolean') view.clear = r.clear;
		out[kind] = view;
	}
	return out;
}

export const SCALE_MIN = 0.8;
export const SCALE_MAX = 1.5;
export const SCALE_STEP = 0.1;

/** a scale kept on the steps and inside the range */
export const clampScale = (v: number): number => {
	if (!Number.isFinite(v)) return 1;
	return Math.round(Math.min(SCALE_MAX, Math.max(SCALE_MIN, v)) * 10) / 10;
};

/** what the browser kept, every field checked (a missing or broken one: the default) */
export function readSettings(raw: unknown): MachineSettings {
	const v = (typeof raw === 'object' && raw !== null ? raw : {}) as Record<string, unknown>;
	const bool = (key: keyof MachineSettings) => (typeof v[key] === 'boolean' ? (v[key] as boolean) : (DEFAULTS[key] as boolean));
	return {
		theme: v.theme === 'night' ? 'night' : 'day',
		colorblind: bool('colorblind'),
		scale: typeof v.scale === 'number' ? clampScale(v.scale) : DEFAULTS.scale,
		panelTransitions: bool('panelTransitions'),
		reduceMotion: bool('reduceMotion'),
		reopenTabs: bool('reopenTabs'),
		gizmoAlways: bool('gizmoAlways'),
		views: readViews(v.views)
	};
}
