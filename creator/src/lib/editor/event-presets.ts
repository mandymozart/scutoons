import { OnomatopoeiaEffect, type OnomatopoeiaParams } from '@scutoons/content';

/**
 * Onomatopoeia presets (owner 2026-10-03: "looks horrible to understand … a preset would be useful"): how the revealed
 * text comes in, as one choice. Only existing params; the "Advanced" fold has every field.
 */
export type OnoPresetId = 'appear' | 'shiver' | 'pulse' | 'wobble' | 'flash';

export interface OnoPreset {
	id: OnoPresetId;
	params: Omit<OnomatopoeiaParams, 'dialog'>;
}

export const ONO_PRESETS: OnoPreset[] = [
	/** just shows up and stays */
	{ id: 'appear', params: { effect: OnomatopoeiaEffect.NONE } },
	{ id: 'shiver', params: { effect: OnomatopoeiaEffect.SHIVER } },
	{ id: 'pulse', params: { effect: OnomatopoeiaEffect.PULSE } },
	{ id: 'wobble', params: { effect: OnomatopoeiaEffect.WOBBLE } },
	/** a short shiver, then gone again */
	{ id: 'flash', params: { effect: OnomatopoeiaEffect.SHIVER, duration: 800, stay: false } }
];

/** the timing params a preset owns (the text, mode and wait stay) */
const TIMING = ['effect', 'duration', 'delay', 'stay'] as const;

export function applyOnoPreset(params: Record<string, unknown>, preset: OnoPreset) {
	for (const key of TIMING) delete params[key];
	Object.assign(params, structuredClone(preset.params));
}

/** the preset the params match exactly (defaults: effect none, duration 2000, delay 0, stays) */
export function matchOnoPreset(params: Record<string, unknown> | null | undefined): OnoPresetId | null {
	const norm = (p: Record<string, unknown> | null | undefined) => ({
		effect: p?.effect ?? OnomatopoeiaEffect.NONE,
		duration: p?.duration ?? 2000,
		delay: p?.delay ?? 0,
		stay: p?.stay ?? true
	});
	const have = norm(params);
	return ONO_PRESETS.find((preset) => {
		const want = norm(preset.params as Record<string, unknown>);
		return TIMING.every((k) => have[k] === want[k]);
	})?.id ?? null;
}
