// model.ts (#773): a model file's facts (deck-space `model_facts`) from the wasm the page loads – the bones, clips,
// spots and what the file lacks, for the creator's 3D place. No rule here.
import { spaceModule } from './space.ts';

export interface ModelBone {
	name: string;
	/** the nearest joint above it (null: a root of the skeleton) */
	parent: string | null;
}

export interface ModelSpot {
	name: string;
	kind: string;
	at: [number, number, number];
}

export interface ModelFacts {
	triangles: number;
	bones: ModelBone[];
	clips: { name: string; seconds: number }[];
	spots: ModelSpot[];
	/** the core's sentences for what the file lacks (none: nothing to say) */
	missing: string[];
	/** #817: its joints, the joints its first clip turns, its clips, its family – what the API stores on the asset version */
	skeleton: Skeleton;
	/** a skeleton and clips with nothing drawn: an animation asset */
	animation: boolean;
}

/** deck-catalog's `Skeleton` */
export interface Skeleton {
	family?: string;
	joints: string[];
	animated?: string[];
	clips?: string[];
}

/** what a character's idle names, checked against its model's skeleton (deck-catalog `FitCheck`) */
export interface IdleCheck {
	animation?: Skeleton;
	clip?: string;
	model: Skeleton;
}

export type ModelAnswer = ({ ok: true } & ModelFacts) | { ok: false; why: string };

interface ModelExports {
	model_facts(bytes: Uint8Array): string;
	idle_problems(check: string): string;
}

/** the core's reading of model files from the wasm at `base` */
export async function loadModelFacts(base: string): Promise<(bytes: Uint8Array) => ModelAnswer> {
	const mod = (await spaceModule(base)) as unknown as ModelExports;
	return (bytes) => JSON.parse(mod.model_facts(bytes)) as ModelAnswer;
}

/** #817: the core's problems of an idle on a model (none: it plays) – the picker offers only what has none */
export async function loadIdleProblems(base: string): Promise<(check: IdleCheck) => string[]> {
	const mod = (await spaceModule(base)) as unknown as ModelExports;
	return (check) => JSON.parse(mod.idle_problems(JSON.stringify(check))) as string[];
}
