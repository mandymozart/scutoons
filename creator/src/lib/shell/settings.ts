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
}

export const DEFAULTS: MachineSettings = { theme: 'day', scale: 1, panelTransitions: true, reduceMotion: false, reopenTabs: true };

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
		reopenTabs: bool('reopenTabs')
	};
}
