// The Generate canvas's stub (agents/research/redesign/AI-HOOKS.md §0): a generator is a weave of spores ending in one
// output spore, whose format says what it makes. Nothing runs and nothing is saved yet – no generator documents, no
// drivers; the gen-AI spores are where external tools hook in. Pure: test/shell.test.ts.

/** what an output spore makes (its format decides which slots take the generator – the core's `takes`, not checked here) */
export type OutputFormat = 'picture' | 'model' | 'sound' | 'video' | 'text';
export const OUTPUT_FORMATS: readonly OutputFormat[] = ['picture', 'model', 'sound', 'video', 'text'];

/** a spore to weave in: our own (the Library, a character's sheet, a crop …) or a gen-AI one (an external tool's hook) */
export interface SporeKind {
	id: string;
	icon: string;
	/** an external gen-AI tool runs it: a hook point */
	hook: boolean;
}

export const OUR_SPORES: readonly SporeKind[] = [
	{ id: 'library', icon: 'grid', hook: false },
	{ id: 'sheet', icon: 'user', hook: false },
	{ id: 'crop', icon: 'crop', hook: false },
	{ id: 'pose', icon: 'move', hook: false },
	{ id: 'condition', icon: 'check-circle', hook: false },
	{ id: 'skills', icon: 'file-text', hook: false },
	{ id: 'reference', icon: 'image', hook: false }
];

export const GEN_SPORES: readonly SporeKind[] = [
	{ id: 'image', icon: 'image', hook: true },
	{ id: 'upscale', icon: 'maximize', hook: true },
	{ id: 'video', icon: 'film', hook: true },
	{ id: 'model3d', icon: 'box', hook: true },
	{ id: 'rig', icon: 'git-branch', hook: true },
	{ id: 'voice', icon: 'mic', hook: true }
];

export const sporeKind = (id: string): SporeKind | undefined => [...OUR_SPORES, ...GEN_SPORES].find((k) => k.id === id);

/** a spore placed on the canvas */
export interface Spore {
	id: string;
	kind: string;
}

/** a spore of `kind` woven in after the others (an unknown kind: unchanged); ids stay unique */
export function addSpore(spores: readonly Spore[], kind: string): Spore[] {
	if (!sporeKind(kind)) return [...spores];
	let n = spores.filter((s) => s.kind === kind).length + 1;
	while (spores.some((s) => s.id === `${kind}-${n}`)) n++;
	return [...spores, { id: `${kind}-${n}`, kind }];
}

export const removeSpore = (spores: readonly Spore[], id: string): Spore[] => spores.filter((s) => s.id !== id);

/** the hook points among the placed spores (where an external gen-AI tool would run) */
export const hooksOf = (spores: readonly Spore[]): number => spores.filter((s) => sporeKind(s.kind)?.hook).length;
