import { TextBoxEffect, type ShowTextBoxParams } from '@scutoons/content';

/**
 * Reveal presets of a showTextBox event (owner 2026-10-03: "looks horrible to understand … a preset would be useful"): how the revealed
 * text comes in, as one choice. Only existing params; the "Advanced" fold has every field.
 */
export type RevealPresetId = 'appear' | 'shiver' | 'pulse' | 'wobble' | 'flash' | 'pop' | 'slam' | 'fade' | 'typeOn';

export interface RevealPreset {
	id: RevealPresetId;
	params: Omit<ShowTextBoxParams, 'dialog'>;
}

export const REVEAL_PRESETS: RevealPreset[] = [
	/** just shows up and stays */
	{ id: 'appear', params: { effect: TextBoxEffect.NONE } },
	{ id: 'shiver', params: { effect: TextBoxEffect.SHIVER } },
	{ id: 'pulse', params: { effect: TextBoxEffect.PULSE } },
	{ id: 'wobble', params: { effect: TextBoxEffect.WOBBLE } },
	/** a short shiver, then gone again */
	{ id: 'flash', params: { effect: TextBoxEffect.SHIVER, duration: 800, stay: false } },
	/** entrances (agents/FX.md): come in once, with their own sound */
	{ id: 'pop', params: { effect: TextBoxEffect.POP } },
	{ id: 'slam', params: { effect: TextBoxEffect.SLAM } },
	{ id: 'fade', params: { effect: TextBoxEffect.FADE } },
	{ id: 'typeOn', params: { effect: TextBoxEffect.TYPE_ON } }
];

/** the timing params a preset owns (the text, mode and wait stay) */
const TIMING = ['effect', 'duration', 'delay', 'stay'] as const;

export function applyRevealPreset(params: Record<string, unknown>, preset: RevealPreset) {
	for (const key of TIMING) delete params[key];
	Object.assign(params, structuredClone(preset.params));
}

/** the preset the params match exactly (defaults: effect none, duration 2000, delay 0, stays) */
export function matchRevealPreset(params: Record<string, unknown> | null | undefined): RevealPresetId | null {
	const norm = (p: Record<string, unknown> | null | undefined) => ({
		effect: p?.effect ?? TextBoxEffect.NONE,
		duration: p?.duration ?? 2000,
		delay: p?.delay ?? 0,
		stay: p?.stay ?? true
	});
	const have = norm(params);
	return REVEAL_PRESETS.find((preset) => {
		const want = norm(preset.params as Record<string, unknown>);
		return TIMING.every((k) => have[k] === want[k]);
	})?.id ?? null;
}
