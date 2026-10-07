/**
 * The reader's effect sounds (agents/FX.md): short sounds the reader synthesizes (@scutoons/reader fx/sounds.ts) – no
 * files. A text box's reveal (`ShowTextBoxParams.sound`) and the `fx` event name one; `none` = silent.
 */
export const FX_SOUNDS = ['pling', 'whoosh', 'thunk', 'tick', 'pop', 'thud', 'soft', 'key', 'zap', 'boom', 'hum'] as const;
export type FxSoundName = (typeof FX_SOUNDS)[number];

/** a sound choice in content: one of the sounds, or `none` (none set = the effect's own) */
export type FxSoundChoice = FxSoundName | 'none';

/** the `fx` event's impacts (agents/FX.md): a shake, a white flash, a quick zoom in and out, dark edges closing in */
export const FX_IMPACTS = ['shake', 'flash', 'punch', 'vignette'] as const;
export type FxImpact = (typeof FX_IMPACTS)[number];

/** the longest an impact may play (ms) */
export const FX_MAX_DURATION = 3000;

/**
 * How the reader highlights a chapter's tap / hover / sound areas while "Show hints" is on (owner 2026-10-08: chosen
 * in the editor, a list to extend): `fireflies` – warm round glows drifting over the area (the default), `alienshine`
 * – brush strokes and specks from the owner's attention.svg, each one random.
 */
export const HIGHLIGHTERS = ['fireflies', 'alienshine'] as const;
export type Highlighter = (typeof HIGHLIGHTERS)[number];

export function isFxSoundChoice(value: unknown): value is FxSoundChoice {
	return value === 'none' || (typeof value === 'string' && (FX_SOUNDS as readonly string[]).includes(value));
}
