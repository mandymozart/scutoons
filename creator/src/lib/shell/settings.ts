// This machine's settings of the creator (research/redesign/PLAN.md §2.5, the deck's settings.rs): the look, motion,
// the layout – kept in this browser. Member settings (language, AI, notifications) live with the member. Pure:
// test/shell.test.ts.

/** where this browser keeps them */
export const SETTINGS_KEY = 'sc-creator-settings';

export type Theme = 'night' | 'day';

export interface MachineSettings {
	theme: Theme;
	/** the UI size: 1rem as a share of the default (0.8 … 1.5) */
	scale: number;
	panelTransitions: boolean;
	reduceMotion: boolean;
	/** the last tabs come back when the creator starts */
	reopenTabs: boolean;
	/** per workspace: the tab on the stage, and whether its content keeps clear of the bar's row */
	views: Record<string, ViewSettings>;
}

export interface ViewSettings {
	/** the tab that took the stage (◱ main); absent: the preset's */
	main?: string | null;
	/** the content starts under the bar's row (the stage itself still runs under the bar); default on */
	clear?: boolean;
}

export const DEFAULTS: MachineSettings = { theme: 'day', scale: 1, panelTransitions: true, reduceMotion: false, reopenTabs: true, views: {} };

/** a workspace's view settings, every field checked */
export function readViews(raw: unknown): Record<string, ViewSettings> {
	if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return {};
	const out: Record<string, ViewSettings> = {};
	for (const [kind, v] of Object.entries(raw as Record<string, unknown>)) {
		if (typeof v !== 'object' || v === null) continue;
		const r = v as Record<string, unknown>;
		const view: ViewSettings = {};
		if (typeof r.main === 'string' || r.main === null) view.main = r.main as string | null;
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
		scale: typeof v.scale === 'number' ? clampScale(v.scale) : DEFAULTS.scale,
		panelTransitions: bool('panelTransitions'),
		reduceMotion: bool('reduceMotion'),
		reopenTabs: bool('reopenTabs'),
		views: readViews(v.views)
	};
}
