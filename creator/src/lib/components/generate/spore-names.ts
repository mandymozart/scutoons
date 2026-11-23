import { m } from '$lib/paraglide/messages.js';
import type { OutputFormat } from '$lib/editor/spores.ts';

/** a spore kind's name (the Spores tab, the canvas) */
const SPORES: Record<string, () => string> = {
	library: () => m['shell.gen_spore_library'](),
	sheet: () => m['shell.gen_spore_sheet'](),
	crop: () => m['shell.gen_spore_crop'](),
	pose: () => m['shell.gen_spore_pose'](),
	condition: () => m['shell.gen_spore_condition'](),
	skills: () => m['shell.gen_spore_skills'](),
	reference: () => m['shell.gen_spore_reference'](),
	image: () => m['shell.gen_spore_image'](),
	upscale: () => m['shell.gen_spore_upscale'](),
	video: () => m['shell.gen_spore_video'](),
	model3d: () => m['shell.gen_spore_model3d'](),
	rig: () => m['shell.gen_spore_rig'](),
	voice: () => m['shell.gen_spore_voice']()
};
export const sporeName = (kind: string): string => SPORES[kind]?.() ?? kind;

const FORMATS: Record<OutputFormat, () => string> = {
	picture: () => m['shell.gen_format_picture'](),
	model: () => m['shell.gen_format_model'](),
	sound: () => m['shell.gen_format_sound'](),
	video: () => m['shell.gen_format_video'](),
	text: () => m['shell.gen_format_text']()
};
export const formatName = (format: OutputFormat): string => FORMATS[format]();

/** what a hook mark says: a hooked spore (an agent, an LLM or an external tool runs it), the output (a run's result goes to a slot), a driver (what starts a run) */
export const hookNote = (where: 'spore' | 'output' | 'driver'): string =>
	where === 'spore' ? m['shell.gen_hook_spore_note']() : where === 'output' ? m['shell.gen_hook_output_note']() : m['shell.gen_hook_driver_note']();
