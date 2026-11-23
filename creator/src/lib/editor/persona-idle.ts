// A persona's idle (#817) as the Rig tab's picker offers it: the clips of its own model and the library's animations,
// each kept only when the core says it plays on the model (deck-catalog `FitCheck`, asked through the wasm – the rule
// the API checks on save and the stage at load). No rule here: the choices, the field's value and its document form.
import type { PersonaAnimation } from '@scutoons/content';
import type { IdleCheck, Skeleton } from './core-model.ts';

export interface IdleChoice {
	kind: 'clip' | 'animation';
	/** the clip's name, the animation's CDN path */
	value: string;
	label: string;
}

/** a library animation as the list answers it (`skeleton` none: not read yet) */
export interface LibraryAnimation {
	path: string;
	title: string;
	skeleton?: Skeleton;
}

export type IdleRule = (check: IdleCheck) => Promise<string[]>;

/** what the picker offers on `model`: its own clips, then the library animations that play on it – the core decides each */
export async function idleChoices(model: Skeleton, animations: LibraryAnimation[], rule: IdleRule): Promise<{ clips: IdleChoice[]; library: IdleChoice[] }> {
	const clips: IdleChoice[] = [];
	for (const name of model.clips ?? []) if ((await rule({ clip: name, model })).length === 0) clips.push({ kind: 'clip', value: name, label: name });
	const library: IdleChoice[] = [];
	for (const a of animations) {
		if (a.skeleton && (await rule({ animation: a.skeleton, model })).length === 0) library.push({ kind: 'animation', value: a.path, label: a.title || a.path });
	}
	return { clips, library };
}

/** the picker's value of a stored idle: `clip:<name>`, `animation:<path>`, '' = none (the model's first clip) */
export function idleValue(idle: PersonaAnimation | undefined): string {
	if (idle?.clip) return `clip:${idle.clip}`;
	if (idle?.animation) return `animation:${idle.animation}`;
	return '';
}

/** the stored idle of a picker's value (undefined = none) */
export function idleOf(value: string): PersonaAnimation | undefined {
	if (value.startsWith('clip:')) return { clip: value.slice(5) };
	if (value.startsWith('animation:')) return { animation: value.slice(10) };
	return undefined;
}
